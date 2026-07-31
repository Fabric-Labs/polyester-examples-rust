//! Prepare an API-key Trading-to-Funding withdraw; submit only when opted in.

use polyester::codecs::scalars::LEDGER_SCALE;
use polyester::models::CreateApiKeyTradingWithdrawParams;
use polyester::{AssetAmount, QuantityDomain};
use polyester_examples::{
    EXAMPLES_ENABLE_WITHDRAWALS_ENV, client_from_env, format_decimal, load_settings,
    pick_usdt_zipper_asset, require_withdrawals_enabled, unique_client_order_id, wait_for_catalogs,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = load_settings();
    let client = client_from_env(true)?;
    wait_for_catalogs(&client).await?;

    let zipper = client.zipper.get_deposit_withdraw_config().await?;
    let asset = pick_usdt_zipper_asset(&zipper).ok_or_else(|| {
        anyhow::anyhow!("USDT / ledger_id=1 not found in zipper deposit-withdraw config")
    })?;

    let amount = format_decimal(settings.withdraw_amount);
    let idempotency_key = unique_client_order_id("wd-prep");

    println!(
        "Preparing API-key Trading->Funding withdraw: asset_id={} amount={amount}",
        asset.ledger_id
    );

    let prepared =
        client
            .withdraw
            .prepare_api_key_to_funding(CreateApiKeyTradingWithdrawParams {
                asset_id: asset.ledger_id,
                amount: AssetAmount::from_decimal_str(
                    &amount,
                    LEDGER_SCALE,
                    QuantityDomain::LedgerE18,
                    Some(asset.ledger_id),
                )?,
                destination_address: String::new(),
                idempotency_key,
                amount_scale: Some(LEDGER_SCALE),
                deadline_ts_sec: None,
                nonce: None,
            })?;
    let payload = prepared.payload();
    println!(
        "Prepared: asset_id={} deadline_ts_sec={} idempotency_key={} request_bytes={}",
        payload.asset_id,
        payload.deadline_ts_sec,
        payload.idempotency_key,
        prepared.request_bytes().len()
    );

    if !settings.enable_withdrawals {
        println!(
            "Submit skipped (set {EXAMPLES_ENABLE_WITHDRAWALS_ENV}=1 to call submit_prepared)"
        );
        return Ok(());
    }

    require_withdrawals_enabled(&settings)?;
    let result = client.withdraw.submit_prepared(&prepared).await?;
    println!(
        "Submitted: intent_id={} status={} flow_id={}",
        result.intent_id, result.status, result.flow_id
    );
    Ok(())
}
