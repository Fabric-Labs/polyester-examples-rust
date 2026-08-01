//! Batch create two post-only limits, then batch_cancel by client order id.

use polyester::models::{
    BatchCancelItem, CreateOrderParams, CreateOrderType, CreateSide, CreateTimeInForce, OrderKey,
};
use polyester::proto::ledger::read::v1::GetBalancesRequest;
use polyester::{Price, Quantity};
use polyester_examples::{
    available_trading_balance, buy_qty_for_quote_cap, cancel_owned_orders_with_prefix,
    client_from_env, load_settings, pair_for_symbol, pick_symbol, quantity_scale_for_symbol,
    quote_asset_id, require_trading_enabled, resolve_post_only_buy_limit_price,
    unique_client_order_id, wait_for_catalogs,
};
use rust_decimal::Decimal;
use std::str::FromStr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = load_settings();
    require_trading_enabled(&settings)?;

    let client = client_from_env(true)?;
    wait_for_catalogs(&client).await?;

    let spot = client.market_data.get_spot_config().await?;
    let symbol = pick_symbol(&spot, &settings.symbol);
    let pair = pair_for_symbol(&spot, &symbol);
    let scale = quantity_scale_for_symbol(&client, &symbol)?;

    let price = resolve_post_only_buy_limit_price(&client, &symbol, pair.as_ref()).await?;
    let asset_id = quote_asset_id(&client, pair.as_ref(), &symbol)
        .ok_or_else(|| anyhow::anyhow!("could not resolve quote asset id for {symbol}"))?;
    let balances = client.balances.list(GetBalancesRequest::default()).await?;
    let available = available_trading_balance(&balances.balances, asset_id);
    let per_order_cap = settings.max_quote / Decimal::from(2);
    let qty = buy_qty_for_quote_cap(
        available,
        per_order_cap,
        Decimal::from_str(&price)?,
        pair.as_ref(),
    )?;

    let cleanup_prefix = "example-bcancel";
    let client_order_ids = [
        unique_client_order_id(cleanup_prefix),
        unique_client_order_id(cleanup_prefix),
    ];
    println!("Batch create 2 post-only buys, then Orders.BatchCancel by client_order_id");

    let items: Vec<CreateOrderParams> = client_order_ids
        .iter()
        .map(|client_order_id| {
            Ok(CreateOrderParams {
                symbol: symbol.clone(),
                side: CreateSide::Buy,
                order_type: CreateOrderType::Limit,
                quantity: Some(Quantity::from_decimal_str(&qty, scale, Some(symbol.clone()), None)?),
            max_quote_debit_scaled: None,
                price: Some(Price::from_decimal_str(&price, Some(symbol.clone()))?),
                time_in_force: Some(CreateTimeInForce::Gtc),
                client_order_id: Some(client_order_id.clone()),
                subaccount_id: None,
                post_only: Some(true),
                market_client_ref_price: None,
                fee_asset: None,
                self_trade_prevention: None,
                market_max_slippage: None,
                attached_risk: None,
            })
        })
        .collect::<anyhow::Result<_>>()?;

    let created = client.orders.batch_create(items, None, None).await?;
    println!(
        "Batch create: accepted={} rejected={}",
        created.accepted_count, created.rejected_count
    );
    if created.accepted_count == 0 {
        anyhow::bail!("no batch orders were accepted");
    }

    let cancel_items = client_order_ids
        .iter()
        .map(|client_order_id| BatchCancelItem {
            key: OrderKey::ClientOrderId(client_order_id.clone()),
            symbol_id: None,
        })
        .collect();
    let canceled = client.orders.batch_cancel(cancel_items, None, None).await?;
    println!(
        "Batch cancel: accepted={} rejected={}",
        canceled.accepted_count, canceled.rejected_count
    );
    for item in &canceled.results {
        let code = if item.code.is_empty() {
            "-"
        } else {
            &item.code
        };
        println!(
            "  client_order_id={} status={} order_id={} code={code}",
            item.client_order_id, item.status, item.order_id
        );
    }

    cancel_owned_orders_with_prefix(&client, cleanup_prefix).await?;
    println!("BatchCancel demo complete; residual owned orders cleaned up");
    Ok(())
}
