//! Batch create two post-only limits, then batch_replace their price.

use polyester::models::{
    BatchReplaceItem, CancelOrderParams, CreateOrderParams, CreateOrderType, CreateSide,
    CreateTimeInForce, OrderKey, is_batch_replace_settled,
};
use polyester::proto::ledger::read::v1::GetBalancesRequest;
use polyester::{Price, Quantity};
use polyester_examples::{
    available_trading_balance, buy_qty_for_quote_cap, client_from_env, load_settings,
    pair_for_symbol, pick_symbol, quantity_scale_for_symbol, quote_asset_id,
    require_trading_enabled, resolve_post_only_buy_limit_price, slightly_lower_limit_price,
    unique_client_order_id, wait_for_catalogs,
};
use rust_decimal::Decimal;
use std::str::FromStr;
use tokio::time::{Duration, Instant, sleep};

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
    let new_price = slightly_lower_limit_price(&price, pair.as_ref())?;
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

    let cleanup_prefix = "example-brepl";
    let predecessor_client_order_ids = [
        unique_client_order_id(cleanup_prefix),
        unique_client_order_id(cleanup_prefix),
    ];
    let successor_client_order_ids = [
        unique_client_order_id(cleanup_prefix),
        unique_client_order_id(cleanup_prefix),
    ];
    let mut cleanup_client_order_ids = predecessor_client_order_ids.clone();
    println!(
        "Batch create 2 post-only buys, then batch_replace new_price={new_price} (was {price})"
    );

    let items: Vec<CreateOrderParams> = predecessor_client_order_ids
        .iter()
        .map(|client_order_id| {
            Ok(CreateOrderParams {
                symbol: symbol.clone(),
                side: CreateSide::Buy,
                order_type: CreateOrderType::Limit,
                quantity: Some(Quantity::from_decimal_str(
                    &qty,
                    scale,
                    Some(symbol.clone()),
                    None,
                )?),
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

    let replace_price = Price::from_decimal_str(&new_price, Some(symbol.clone()))?;
    let replace_items = predecessor_client_order_ids
        .iter()
        .enumerate()
        .map(|(index, client_order_id)| BatchReplaceItem {
            key: OrderKey::ClientOrderId(client_order_id.clone()),
            new_price: Some(replace_price.clone()),
            new_qty: None,
            new_attached_risk: None,
            new_client_order_id: Some(successor_client_order_ids[index].clone()),
        })
        .collect();
    let request_id = unique_client_order_id("example-brepl-request");
    let replaced = client
        .orders
        .batch_replace(replace_items, &symbol, None, Some(request_id.clone()))
        .await?;
    // If this request timed out ambiguously, retry the exact same logical batch
    // with request_id. Do not generate a new ID for an idempotent retry.
    // let retry = client.orders.batch_replace(replace_items, &symbol, None, Some(request_id)).await?;
    println!(
        "Batch replace admission: request_id={request_id} batch_request_id={} status={} accepted={} rejected={}",
        replaced.batch_request_id,
        replaced.status,
        replaced.accepted_count,
        replaced.rejected_count
    );
    for item in &replaced.results {
        if let Some(client_order_id) = successor_client_order_ids.get(item.item_index as usize)
            && !item.replacement_order_id.is_empty()
        {
            cleanup_client_order_ids[item.item_index as usize] = client_order_id.clone();
        }
        let code = if item.code.is_empty() {
            "-"
        } else {
            &item.code
        };
        println!(
            "  item={} status={} predecessor_order_id={} replacement_order_id={} successor_client_order_id={} code={code}",
            item.item_index,
            item.status,
            item.old_order_id,
            item.replacement_order_id,
            item.client_order_id
        );
    }
    println!(
        "Predecessor client IDs are stale after admission: {:?}",
        predecessor_client_order_ids
    );
    println!(
        "Cleanup and later tracking use successor client IDs (and retain any rejected predecessor): {:?}",
        cleanup_client_order_ids
    );

    let deadline = Instant::now() + Duration::from_secs_f64(settings.order_timeout_sec);
    loop {
        let status = client
            .orders
            .get_batch_replace_status(&replaced.batch_request_id, None)
            .await?;
        let settled = is_batch_replace_settled(&status);
        println!(
            "Batch replace status: admission={} items={} accepted={} rejected={} settled={settled}",
            status.admission_status,
            status.items.len(),
            status.accepted_count,
            status.rejected_count
        );
        for item in &status.items {
            println!(
                "  item={} phase={} predecessor_order_id={} replacement_order_id={} order_status={}",
                item.item_index,
                item.phase,
                item.old_order_id,
                item.replacement_order_id,
                item.order_status
            );
        }
        if settled {
            break;
        }
        if Instant::now() >= deadline {
            anyhow::bail!(
                "batch replace did not settle within {}s",
                settings.order_timeout_sec
            );
        }
        sleep(Duration::from_secs_f64(settings.poll_sec)).await;
    }

    for client_order_id in cleanup_client_order_ids {
        if let Err(err) = client
            .orders
            .cancel_with(CancelOrderParams {
                key: OrderKey::ClientOrderId(client_order_id.clone()),
                symbol: Some(symbol.clone()),
                symbol_id: None,
                subaccount_id: None,
            })
            .await
        {
            eprintln!("Cleanup warning for {client_order_id}: {err}");
        }
    }
    println!("Targeted cleanup completed for tracked batch-replace orders");
    Ok(())
}
