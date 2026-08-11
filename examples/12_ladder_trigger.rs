//! Create a ladder trigger, read it back, then cancel it.

use polyester::models::{
    CreateOrderType, CreateSide, CreateTimeInForce, CreateTriggerParams, CreateTriggerType,
    ListTriggersOpts,
};
use polyester::{Price, Quantity};
use polyester_examples::{
    cancel_all_for_symbol, client_from_env, format_decimal, load_settings, min_base_qty_for_pair,
    pair_for_symbol, pick_symbol, price_to_ticks, quantity_scale_for_symbol,
    require_trading_enabled, resolve_post_only_buy_limit_price, tick_size, unique_client_order_id,
    wait_for_catalogs,
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

    let far_below = resolve_post_only_buy_limit_price(&client, &symbol, pair.as_ref()).await?;
    let tick_ticks = price_to_ticks(&format_decimal(tick_size(pair.as_ref())))?;
    if tick_ticks <= 0 {
        anyhow::bail!("invalid tick size for ladder alignment");
    }
    let max_ticks = (price_to_ticks(&far_below)? / tick_ticks) * tick_ticks;
    let min_ticks = ((max_ticks * 80 / 100) / tick_ticks).max(1) * tick_ticks;
    let ladder_price_max = Price::from_ticks(max_ticks, Some(symbol.clone()))?;
    let ladder_price_min = Price::from_ticks(min_ticks, Some(symbol.clone()))?;
    let ladder_min_text = ladder_price_min.format();
    let ladder_max_text = ladder_price_max.format();

    let levels = 2;
    let per_level_qty =
        Decimal::from_str(&min_base_qty_for_pair(pair.as_ref(), &ladder_min_text)?)?;
    let total_qty = format_decimal(per_level_qty * Decimal::from(levels));
    let client_trigger_id = unique_client_order_id("trg-ladder");

    println!(
        "Creating ladder trigger: symbol={symbol} levels={levels} min={ladder_min_text} \
         max={ladder_max_text} qty={total_qty} dist=linear"
    );

    let created = client
        .triggers
        .create(CreateTriggerParams {
            symbol: symbol.clone(),
            trigger_type: CreateTriggerType::Ladder,
            side: CreateSide::Buy,
            order_type: CreateOrderType::Limit,
            qty: Quantity::from_decimal_str(&total_qty, scale, Some(symbol.clone()), None)?,
            trigger_price: None,
            limit_price: Some(ladder_price_max.clone()),
            trigger_price_source: None,
            time_in_force: Some(CreateTimeInForce::Gtc),
            subaccount_id: None,
            client_trigger_id,
            post_only: true,
            activation_price: None,
            trailing_distance_ticks: None,
            trailing_distance_bps: None,
            max_slippage_ticks: None,
            max_slippage_bps: None,
            twap_duration_ms: None,
            twap_slice_interval_ms: None,
            ladder_price_min: Some(ladder_price_min),
            ladder_price_max: Some(ladder_price_max),
            ladder_levels: Some(levels),
            ladder_distribution: Some("linear".into()),
            fee_asset: None,
            self_trade_prevention_mode: None,
        })
        .await?;
    println!(
        "Created: trigger_id={} status={}",
        created.trigger_id, created.status
    );

    match client
        .triggers
        .list_with(ListTriggersOpts {
            symbol: Some(symbol.clone()),
            limit: Some(50),
            ..Default::default()
        })
        .await
    {
        Ok(listed) => println!("List: {} trigger(s) for {symbol}", listed.triggers.len()),
        Err(err) => println!("List warning: {err}"),
    }

    if !created.trigger_id.is_empty() {
        match client.triggers.get_by_id(&created.trigger_id, None).await {
            Ok(Some(got)) => println!(
                "Get: trigger_id={} type={} side={} status={} parent_order_id={:?}",
                got.trigger_id, got.trigger_type, got.side, got.status, got.parent_order_id
            ),
            Ok(None) => println!("Get warning: trigger not found"),
            Err(err) => println!("Get warning: {err}"),
        }
        match client
            .triggers
            .cancel_by_id(&created.trigger_id, None)
            .await
        {
            Ok(_) => println!("Trigger canceled"),
            Err(err) => println!("Trigger cancel warning: {err}"),
        }
    }

    cancel_all_for_symbol(&client, &symbol).await;
    println!("Best-effort cancel_all cleanup submitted");
    Ok(())
}
