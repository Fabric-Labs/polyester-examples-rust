//! Encode Funding-to-Trading deposit calldata; optionally submit one UserOp.

use polyester::chain::{
    POLYESTER_TESTNET_ENVIRONMENT, PolyesterSmartAccount, SendCallsResult,
    encode_trading_gateway_deposit,
};
use polyester_examples::{
    EXAMPLES_ENABLE_CHAIN_FUNDING_TO_TRADING_ENV, OWNER_PRIVATE_KEY_ENV, client_from_env,
    human_amount_to_e18, load_settings, pick_usdt_zipper_asset,
    require_chain_funding_to_trading_enabled, require_owner_private_key, wait_for_catalogs,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = load_settings();
    let client = client_from_env(true)?;
    wait_for_catalogs(&client).await?;

    let zipper = client.zipper.get_deposit_withdraw_config().await?;
    let asset = pick_usdt_zipper_asset(&zipper).ok_or_else(|| {
        anyhow::anyhow!("USDT / ledger_id=1 not found in zipper deposit-withdraw config")
    })?;
    let qty = human_amount_to_e18(settings.chain_amount)?;

    let call = encode_trading_gateway_deposit(
        POLYESTER_TESTNET_ENVIRONMENT
            .contracts
            .trading_gateway_address,
        &asset.u_asset_id,
        qty,
    )?;

    println!(
        "Encoded TradingGateway.deposit: to={} u_asset_id={} qty_e18={qty}",
        call.to, asset.u_asset_id
    );
    println!("calldata={}", hex_encode(&call.data));

    if !settings.enable_chain_funding_to_trading {
        println!(
            "Submit skipped (set {EXAMPLES_ENABLE_CHAIN_FUNDING_TO_TRADING_ENV}=1 and \
             {OWNER_PRIVATE_KEY_ENV} to send_calls)"
        );
        return Ok(());
    }

    require_chain_funding_to_trading_enabled(&settings)?;
    require_owner_private_key(&settings)?;

    let account = PolyesterSmartAccount::new(
        &settings.owner_private_key,
        None,
        0,
        Duration::from_secs(60),
    )?;
    println!(
        "Smart account: {} (owner={})",
        account.address, account.owner_address
    );

    let result = account
        .send_calls(&[call], true, Duration::from_secs(120))
        .await?;
    match result {
        SendCallsResult::Receipt(receipt) => println!(
            "UserOp receipt: success={} user_operation_hash={} tx={}",
            receipt.success, receipt.user_operation_hash, receipt.transaction_hash
        ),
        SendCallsResult::Hash(hash) => println!("UserOp hash: {hash}"),
    }
    Ok(())
}

fn hex_encode(data: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(2 + data.len() * 2);
    out.push_str("0x");
    for byte in data {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
