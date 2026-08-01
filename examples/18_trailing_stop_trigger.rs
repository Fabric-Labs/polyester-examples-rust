//! Create a standalone trailing-stop trigger (SELL market-IOC), read it back, then cancel it.
//!
//! POLY-3787: wire `TrailingStopTrigger` carries `side`; list/get project `trigger_type`,
//! `side`, and `parent_order_id` (empty for standalone triggers).

use polyester::models::{
    CreateOrderType, CreateSide, CreateTriggerParams, CreateTriggerType, ListTriggersOpts,
};
use polyester::Quantity;
use polyester_examples::{
    cancel_all_for_symbol, client_from_env, load_settings, min_base_qty_for_pair, pair_for_symbol,
    pick_symbol, quantity_scale_for_symbol, require_trading_enabled,
    resolve_post_only_buy_limit_price, unique_client_order_id, wait_for_catalogs,
};

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

    // Size from the post-only buy helper price; trailing child is market-IOC sell.
    let ref_price = resolve_post_only_buy_limit_price(&client, &symbol, pair.as_ref()).await?;
    let qty = min_base_qty_for_pair(pair.as_ref(), &ref_price)?;
    let client_trigger_id = unique_client_order_id("trg-trail");
    let trailing_distance_bps = 100;

    println!(
        "Creating trailing-stop trigger: symbol={symbol} side=sell qty={qty} \
         trailing_distance_bps={trailing_distance_bps}"
    );

    let created = client
        .triggers
        .create(CreateTriggerParams {
            symbol: symbol.clone(),
            trigger_type: CreateTriggerType::TrailingStop,
            side: CreateSide::Sell,
            order_type: CreateOrderType::Market,
            qty: Quantity::from_decimal_str(&qty, scale, Some(symbol.clone()), None)?,
            trigger_price: None,
            limit_price: None,
            trigger_price_source: None,
            time_in_force: None,
            subaccount_id: None,
            client_trigger_id,
            post_only: false,
            activation_price: None,
            trailing_distance_ticks: None,
            trailing_distance_bps: Some(trailing_distance_bps),
            max_slippage_ticks: None,
            max_slippage_bps: None,
            twap_duration_ms: None,
            twap_slice_interval_ms: None,
            ladder_price_min: None,
            ladder_price_max: None,
            ladder_levels: None,
            ladder_distribution: None,
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
            limit: 50,
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
