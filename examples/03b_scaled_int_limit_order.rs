//! Same flow as `03`, but qty/price stay in wire units (bot path).
//!
//! Demonstrates `Quantity::from_scaled` / `Price::from_ticks` with no decimal
//! string round-trip on the order create call.

use polyester::models::{CreateOrderParams, CreateOrderType, CreateSide, CreateTimeInForce};
use polyester::proto::ledger::read::v1::GetBalancesRequest;
use polyester::{Price, Quantity, QuantityDomain};
use polyester_examples::{
    available_trading_balance, buy_qty_for_quote_cap, cancel_all_for_symbol, cancel_order,
    client_from_env, load_settings, pair_for_symbol, pick_symbol, price_to_ticks, qty_to_scaled,
    quantity_scale_for_symbol, quote_asset_id, require_trading_enabled,
    resolve_post_only_buy_limit_price, unique_client_order_id, wait_for_catalogs,
    wait_for_no_open_order, wait_for_open_order,
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

    // Humans still size from book/decimals; bots convert once, then stay in ints.
    let price_decimal = resolve_post_only_buy_limit_price(&client, &symbol, pair.as_ref()).await?;
    let asset_id = quote_asset_id(&client, pair.as_ref(), &symbol)
        .ok_or_else(|| anyhow::anyhow!("could not resolve quote asset id for {symbol}"))?;
    let balances = client.balances.list(GetBalancesRequest::default()).await?;
    let available = available_trading_balance(&balances.balances, asset_id);
    let qty_decimal = buy_qty_for_quote_cap(
        available,
        settings.max_quote,
        Decimal::from_str(&price_decimal)?,
        pair.as_ref(),
    )?;

    let qty_scaled = qty_to_scaled(&qty_decimal, scale)?;
    let price_ticks = price_to_ticks(&price_decimal)?;
    let client_order_id = unique_client_order_id("example-scaled");

    println!(
        "Creating scaled-int post-only buy: symbol={symbol} price_ticks={price_ticks} \
         qty_scaled={qty_scaled} (scale={scale}) client_order_id={client_order_id}"
    );

    let created = client
        .orders
        .create(CreateOrderParams {
            symbol: symbol.clone(),
            side: CreateSide::Buy,
            order_type: CreateOrderType::Limit,
            quantity: Some(Quantity::from_scaled(
                qty_scaled,
                Some(scale),
                QuantityDomain::OrderBase,
                Some(symbol.clone()),
                None,
            )?),
            max_quote_debit_scaled: None,
            price: Some(Price::from_ticks(price_ticks, Some(symbol.clone()))?),
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
        .await?;
    println!(
        "Created: status={} order_id={}",
        created.status, created.order_id
    );

    let _ = wait_for_open_order(
        &client,
        &client_order_id,
        settings.order_timeout_sec,
        settings.poll_sec,
    )
    .await;
    cancel_order(&client, &symbol, &created.order_id, &client_order_id).await?;
    let _ = wait_for_no_open_order(
        &client,
        &client_order_id,
        settings.order_timeout_sec,
        settings.poll_sec,
    )
    .await;
    cancel_all_for_symbol(&client, &symbol).await;
    println!("Scaled-int order cycle complete");
    Ok(())
}
