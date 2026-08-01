//! Encode Funding-to-external withdraw calldata; optionally submit one UserOp.

use alloy_primitives::U256;
use polyester::chain::{
    POLYESTER_TESTNET_ENVIRONMENT, PolyesterSmartAccount, SendCallsResult,
    encode_funding_withdraw_to_chain, encode_withdraw_destination, quote_zipper_fee,
};
use polyester_examples::{
    EXAMPLES_ENABLE_CHAIN_EXTERNAL_SUBMIT_ENV, EXAMPLES_EXTERNAL_DESTINATION_ENV,
    OWNER_PRIVATE_KEY_ENV, client_from_env, human_amount_to_e18, load_settings,
    pick_usdt_zipper_asset, require_owner_private_key, wait_for_catalogs,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = load_settings();
    if settings.external_destination.trim().is_empty() {
        anyhow::bail!(
            "{EXAMPLES_EXTERNAL_DESTINATION_ENV} is required to encode Funding->external withdraw"
        );
    }

    let client = client_from_env(true)?;
    wait_for_catalogs(&client).await?;

    let zipper = client.zipper.get_deposit_withdraw_config().await?;
    let asset = pick_usdt_zipper_asset(&zipper).ok_or_else(|| {
        anyhow::anyhow!("USDT / ledger_id=1 not found in zipper deposit-withdraw config")
    })?;
    let chain_id = u16::try_from(settings.external_chain_id)
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| {
            anyhow::anyhow!("invalid external chain id: {}", settings.external_chain_id)
        })?;
    let variant = asset
        .variants
        .iter()
        .find(|v| v.chain_id == u32::from(chain_id) && !v.z_token.address.is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!("No USDT z_token variant for withdraw chain_id={chain_id}")
        })?;
    let case_sensitive = zipper
        .chains
        .iter()
        .find(|c| c.chain_id == u32::from(chain_id))
        .map(|c| c.is_case_sensitive)
        .unwrap_or(false);

    let z_amount = human_amount_to_e18(settings.chain_amount)?;
    let fee = quote_zipper_fee(
        chain_id,
        &variant.z_token.address,
        POLYESTER_TESTNET_ENVIRONMENT
            .contracts
            .zipper_endpoint_address,
        None,
        None,
    )
    .await?;
    let max_fee = fee.fee + fee.fee / U256::from(10_u64);
    if z_amount <= max_fee {
        anyhow::bail!(
            "chain amount {z_amount} must be greater than max_fee {max_fee}; raise POLYESTER_EXAMPLES_CHAIN_AMOUNT"
        );
    }

    let dest_bytes = encode_withdraw_destination(&settings.external_destination, case_sensitive);
    let call = encode_funding_withdraw_to_chain(
        POLYESTER_TESTNET_ENVIRONMENT
            .contracts
            .funding_account_address,
        chain_id,
        &variant.z_token.address,
        &dest_bytes,
        z_amount,
        max_fee,
    )?;

    println!(
        "Encoded FundingAccount.withdrawToChain: to={} chain_id={chain_id} z_token={} \
         z_amount={z_amount} max_fee={max_fee} dest={}",
        call.to, variant.z_token.address, settings.external_destination
    );
    println!("calldata={}", hex_encode(&call.data));

    if !settings.enable_chain_external_submit {
        println!(
            "Submit skipped (set {EXAMPLES_ENABLE_CHAIN_EXTERNAL_SUBMIT_ENV}=1 and \
             {OWNER_PRIVATE_KEY_ENV} to send_calls)"
        );
        return Ok(());
    }

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
