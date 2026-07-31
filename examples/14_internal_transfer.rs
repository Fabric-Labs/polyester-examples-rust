//! Submit a tiny internal transfer to another Polyester account.

use polyester::codecs::scalars::LEDGER_SCALE;
use polyester::models::CreateInternalTransferParams;
use polyester::{AssetAmount, QuantityDomain};
use polyester_examples::{
    client_from_env, format_decimal, load_settings, pair_for_symbol, pick_symbol,
    pick_usdt_zipper_asset, quote_asset_id, require_transfers_enabled, unique_client_order_id,
    wait_for_catalogs,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = load_settings();
    require_transfers_enabled(&settings)?;

    let client = client_from_env(true)?;
    wait_for_catalogs(&client).await?;

    let spot = client.market_data.get_spot_config().await?;
    let symbol = pick_symbol(&spot, &settings.symbol);
    let pair = pair_for_symbol(&spot, &symbol);

    let asset_id = match client.zipper.get_deposit_withdraw_config().await {
        Ok(zipper) => pick_usdt_zipper_asset(&zipper).map(|asset| asset.ledger_id),
        Err(_) => None,
    }
    .or_else(|| quote_asset_id(&client, pair.as_ref(), &symbol))
    .ok_or_else(|| {
        anyhow::anyhow!("could not resolve USDT/ledger or quote asset id for transfer")
    })?;

    let amount = format_decimal(settings.transfer_amount);
    let dest = settings.transfer_dest_account_id.trim().to_owned();
    let idempotency_key = unique_client_order_id("xfer");

    println!("Internal transfer: asset_id={asset_id} amount={amount} dest_account={dest}");

    let result = client
        .internal_transfers
        .create(CreateInternalTransferParams {
            asset_id,
            quantity: AssetAmount::from_decimal_str(
                &amount,
                LEDGER_SCALE,
                QuantityDomain::LedgerE18,
                Some(asset_id),
            )?,
            idempotency_key,
            subaccount_id: None,
            destination_account_id: Some(dest),
            destination_subaccount_id: None,
            destination_smart_account_address: None,
            quantity_scale: Some(LEDGER_SCALE),
        })
        .await?;
    println!(
        "Transfer accepted: request_id={} transfer_id={} quantity.scaled={}",
        result.request_id,
        result.transfer_id,
        result
            .quantity
            .as_ref()
            .map(|q| q.as_scaled().to_string())
            .unwrap_or_else(|| "-".into())
    );
    Ok(())
}
