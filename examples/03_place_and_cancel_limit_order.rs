//! Place a post-only buy limit (decimal path), then cancel it.
//!
//! Requires `POLYESTER_EXAMPLES_ENABLE_TRADING=1` and trading balance.

use polyester::models::{CreateOrderParams, CreateOrderType, CreateSide, CreateTimeInForce};
use polyester::proto::ledger::read::v1::GetBalancesRequest;
use polyester::{Price, Quantity};
use polyester_examples::{
    available_trading_balance, buy_qty_for_quote_cap, cancel_all_for_symbol, cancel_order,
    client_from_env, load_settings, pair_for_symbol, pick_symbol, quantity_scale_for_symbol,
    quote_asset_id, require_trading_enabled, resolve_post_only_buy_limit_price,
    unique_client_order_id, wait_for_catalogs, wait_for_no_open_order, wait_for_open_order,
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
    let qty = buy_qty_for_quote_cap(
        available,
        settings.max_quote,
        Decimal::from_str(&price)?,
        pair.as_ref(),
    )?;

    let client_order_id = unique_client_order_id("example-limit");
    println!(
        "Creating post-only buy limit order: symbol={symbol} price={price} qty={qty} \
         client_order_id={client_order_id}"
    );

    let created = match client
        .orders
        .create(CreateOrderParams {
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
        .await
    {
        Ok(c) => c,
        Err(err) => {
            cancel_all_for_symbol(&client, &symbol).await;
            return Err(err.into());
        }
    };
    println!(
        "Created: status={} order_id={}",
        created.status, created.order_id
    );

    match wait_for_open_order(
        &client,
        &client_order_id,
        settings.order_timeout_sec,
        settings.poll_sec,
    )
    .await
    {
        Ok(open) => println!("Visible in open orders: status={}", open.status),
        Err(err) => {
            println!(
                "Order create was accepted, but open-order reads did not catch up in time. \
                 This is a known devnet OMS read-indexing issue — canceling by order_id anyway."
            );
            println!("  detail: {err}");
        }
    }

    if let Err(err) = cancel_order(&client, &symbol, &created.order_id, &client_order_id).await {
        cancel_all_for_symbol(&client, &symbol).await;
        return Err(err);
    }
    println!("Cancel submitted");

    if let Err(err) = wait_for_no_open_order(
        &client,
        &client_order_id,
        settings.order_timeout_sec,
        settings.poll_sec,
    )
    .await
    {
        println!(
            "Cancel was submitted, but open-order reads still show the order. \
             Check the Polyester UI or wait for the devnet read path to catch up."
        );
        println!("  detail: {err}");
    } else {
        println!("Order is no longer open");
    }

    cancel_all_for_symbol(&client, &symbol).await;
    Ok(())
}
