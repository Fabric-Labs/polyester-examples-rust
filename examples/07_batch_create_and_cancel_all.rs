//! Batch create two post-only limits, then flatten with cancel_all.

use polyester::models::{CreateOrderParams, CreateOrderType, CreateSide, CreateTimeInForce};
use polyester::proto::ledger::read::v1::GetBalancesRequest;
use polyester::{Price, Quantity};
use polyester_examples::{
    available_trading_balance, buy_qty_for_quote_cap, cancel_all_for_symbol, client_from_env,
    load_settings, pair_for_symbol, pick_symbol, quantity_scale_for_pair, quote_asset_id,
    require_trading_enabled, resolve_post_only_buy_limit_price, unique_client_order_id,
    wait_for_catalogs, wait_for_open_order,
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
    let scale = quantity_scale_for_pair(pair.as_ref());

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

    let client_order_ids = [
        unique_client_order_id("example-batch-a"),
        unique_client_order_id("example-batch-b"),
    ];
    println!(
        "Batch creating 2 post-only buy limits: symbol={symbol} price={price} qty={qty} each \
         (max ~{per_order_cap} quote per order)"
    );

    let items: Vec<CreateOrderParams> = client_order_ids
        .iter()
        .map(|client_order_id| {
            Ok(CreateOrderParams {
                symbol: symbol.clone(),
                side: CreateSide::Buy,
                order_type: CreateOrderType::Limit,
                quantity: Quantity::from_decimal_str(&qty, scale, Some(symbol.clone()), None)?,
                price: Some(Price::from_decimal_str(&price, Some(symbol.clone()))?),
                time_in_force: Some(CreateTimeInForce::Gtc),
                client_order_id: Some(client_order_id.clone()),
                subaccount_id: None,
                post_only: Some(true),
                market_client_ref_price: None,
                attached_risk: None,
            })
        })
        .collect::<anyhow::Result<_>>()?;

    let created = match client
        .orders
        .batch_create(items, None, None)
        .await
    {
        Ok(c) => c,
        Err(err) => {
            cancel_all_for_symbol(&client, &symbol).await;
            return Err(err.into());
        }
    };
    println!(
        "Batch create: accepted={} rejected={}",
        created.accepted_count, created.rejected_count
    );
    for item in &created.results {
        let code = if item.code.is_empty() {
            "-"
        } else {
            item.code.as_str()
        };
        println!(
            "  client_order_id={} status={} order_id={} code={code}",
            item.client_order_id, item.status, item.order_id
        );
    }
    if created.accepted_count == 0 {
        cancel_all_for_symbol(&client, &symbol).await;
        anyhow::bail!("no batch orders were accepted");
    }

    for client_order_id in &client_order_ids {
        match wait_for_open_order(
            &client,
            client_order_id,
            settings.order_timeout_sec,
            settings.poll_sec,
        )
        .await
        {
            Ok(open) => println!(
                "Visible in open orders: {client_order_id} status={}",
                open.status
            ),
            Err(err) => println!(
                "  {client_order_id}: create accepted but open-order reads lagged ({err})"
            ),
        }
    }

    let canceled = client.orders.cancel_all(Some(&symbol), false, None).await?;
    println!(
        "cancel_all: status={} matched_orders={} submitted_cancels={}",
        canceled.status, canceled.matched_orders, canceled.submitted_cancels
    );

    cancel_all_for_symbol(&client, &symbol).await;
    Ok(())
}
