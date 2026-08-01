//! Preview an order intent without creating it (admissibility + protected bound).

use polyester::models::{CreateOrderType, CreateSide, CreateTimeInForce, PreviewOrderParams};
use polyester::proto::ledger::read::v1::GetBalancesRequest;
use polyester::{Price, Quantity};
use polyester_examples::{
    available_trading_balance, buy_qty_for_quote_cap, client_from_env, load_settings,
    min_base_qty_for_pair, pair_for_symbol, pick_symbol, quantity_scale_for_symbol, quote_asset_id,
    resolve_post_only_buy_limit_price, wait_for_catalogs,
};
use rust_decimal::Decimal;
use std::str::FromStr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = load_settings();
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
    let qty = match buy_qty_for_quote_cap(
        available,
        settings.max_quote,
        Decimal::from_str(&price)?,
        pair.as_ref(),
    ) {
        Ok(qty) => qty,
        Err(_) => {
            let qty = min_base_qty_for_pair(pair.as_ref(), &price)?;
            println!(
                "Insufficient quote trading balance (available={available}); \
                 previewing minimum qty={qty} to exercise admission rejection"
            );
            qty
        }
    };

    println!(
        "Previewing post-only buy limit: symbol={symbol} price={price} qty={qty} \
         (no order is created)"
    );

    let preview = match client
        .orders
        .preview(PreviewOrderParams {
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
            client_order_id: None,
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
        Ok(preview) => preview,
        Err(err) => {
            let message = err.to_string().to_lowercase();
            if message.contains("unimplemented")
                || message.contains("not found")
                || message.contains("not exposed")
            {
                println!("PreviewOrder is not available on this API host ({err}). Skipping.");
                return Ok(());
            }
            return Err(err.into());
        }
    };

    println!("  admissible={:?}", preview.admissible);
    println!(
        "  resolved_base_qty.scaled={:?}",
        preview
            .resolved_base_qty
            .as_ref()
            .map(|qty| qty.as_scaled())
    );
    if let Some(bound) = &preview.protected_price_bound {
        println!("  protected_price_bound.ticks={}", bound.as_ticks());
    }
    println!("  evaluated_at_ms={}", preview.evaluated_at_ms);
    if let Some(rejection) = &preview.rejection {
        println!("  rejection.code={}", rejection.code);
        for violation in &rejection.violations {
            println!(
                "  violation field={} rule={} message={}",
                violation.field_path, violation.rule_id, violation.message
            );
        }
    }

    Ok(())
}
