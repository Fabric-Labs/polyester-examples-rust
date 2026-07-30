//! Candles + RSI signal; optional small live limit order.

use polyester::models::{CreateOrderParams, CreateOrderType, CreateSide, CreateTimeInForce};
use polyester::proto::ledger::read::v1::GetBalancesRequest;
use polyester::{Price, Quantity};
use polyester_examples::{
    available_trading_balance, base_asset_id, buy_qty_for_quote_cap, cancel_after_timeout,
    cancel_all_for_symbol, client_from_env, ensure_no_open_orders_with_prefix, format_decimal,
    load_settings, marketable_limit_price, pair_for_symbol, pick_symbol, quantity_scale_for_pair,
    quote_asset_id, quote_asset_symbol, rsi_signal, sell_qty_for_quote_cap, unique_client_order_id,
    wait_for_catalogs,
};
use rust_decimal::Decimal;
use std::str::FromStr;

const BOT_PREFIX: &str = "rsi-bot";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = load_settings();
    let client = client_from_env(settings.enable_trading)?;
    wait_for_catalogs(&client).await?;

    let spot = client.market_data.get_spot_config().await?;
    let symbol = pick_symbol(&spot, &settings.symbol);
    let pair = pair_for_symbol(&spot, &symbol);
    let scale = quantity_scale_for_pair(pair.as_ref());

    let limit = settings.candle_limit.max(settings.rsi_period + 2);
    let candles = client
        .market_data
        .get_candles(&symbol, &settings.timeframe, Some(limit as u32))
        .await?;
    let closes: Vec<Decimal> = candles
        .candles
        .iter()
        .filter_map(|c| Decimal::from_str(c.close.trim()).ok())
        .collect();
    if closes.is_empty() {
        anyhow::bail!("no candles returned for {symbol}");
    }

    let signal = rsi_signal(
        &closes,
        settings.rsi_period,
        settings.rsi_oversold,
        settings.rsi_overbought,
    )?;
    let last_close = *closes.last().unwrap();
    println!(
        "{symbol} {}: close={} rsi={} previous_rsi={} action={} reason={}",
        settings.timeframe,
        format_decimal(last_close),
        fmt_rsi(signal.latest_rsi),
        fmt_rsi(signal.previous_rsi),
        signal.action,
        signal.reason
    );

    if signal.action == "hold" {
        return Ok(());
    }

    let price = marketable_limit_price(&signal.action, last_close, pair.as_ref())?;
    println!(
        "Signal order candidate: side={} price={price}",
        signal.action
    );

    if !settings.enable_trading {
        println!(
            "Dry run only. Set POLYESTER_EXAMPLES_ENABLE_TRADING=1 to allow \
             the bot to place and manage this order."
        );
        return Ok(());
    }

    ensure_no_open_orders_with_prefix(&client, BOT_PREFIX).await?;
    let balances = client.balances.list(GetBalancesRequest::default()).await?;
    let qty = qty_for_signal(
        &client,
        &balances.balances,
        &symbol,
        pair.as_ref(),
        &signal.action,
        Decimal::from_str(&price)?,
        settings.max_quote,
    )?;
    let client_order_id = unique_client_order_id(BOT_PREFIX);
    let side = match signal.action.as_str() {
        "buy" => CreateSide::Buy,
        "sell" => CreateSide::Sell,
        other => anyhow::bail!("unsupported side {other}"),
    };

    println!(
        "Placing live limit order: side={} price={price} qty={qty} client_order_id={client_order_id}",
        signal.action
    );

    let created = match client
        .orders
        .create(CreateOrderParams {
            symbol: symbol.clone(),
            side,
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
            post_only: Some(false),
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

    let final_status = cancel_after_timeout(
        &client,
        &client_order_id,
        &symbol,
        &created.order_id,
        settings.order_timeout_sec,
        settings.poll_sec,
    )
    .await?;
    println!("Final observed status: {final_status}");
    if final_status == "canceled_after_timeout_unconfirmed" {
        println!(
            "Cancel was submitted, but open-order reads did not confirm cleanup. \
             This can happen on devnet when OMS read indexing lags."
        );
    }

    cancel_all_for_symbol(&client, &symbol).await;
    Ok(())
}

fn qty_for_signal(
    client: &polyester::Client,
    balances: &[polyester::models::AssetBalance],
    symbol: &str,
    pair: Option<&polyester::proto::marketdata::v1::PairConfig>,
    side: &str,
    price: Decimal,
    max_quote: Decimal,
) -> anyhow::Result<String> {
    match side {
        "buy" => {
            let asset_id = quote_asset_id(client, pair, symbol)
                .ok_or_else(|| anyhow::anyhow!("could not resolve quote asset id for {symbol}"))?;
            let available_quote = available_trading_balance(balances, asset_id);
            let qty = buy_qty_for_quote_cap(available_quote, max_quote, price, pair)?;
            println!(
                "Using up to {} {} of trading balance",
                format_decimal(max_quote),
                quote_asset_symbol(pair, symbol)
            );
            Ok(qty)
        }
        "sell" => {
            let asset_id = base_asset_id(client, pair, symbol)
                .ok_or_else(|| anyhow::anyhow!("could not resolve base asset id for {symbol}"))?;
            let available_base = available_trading_balance(balances, asset_id);
            sell_qty_for_quote_cap(available_base, max_quote, price, pair)
        }
        _ => anyhow::bail!("side must be 'buy' or 'sell'"),
    }
}

fn fmt_rsi(value: Option<Decimal>) -> String {
    match value {
        Some(v) => format_decimal(v.round_dp(2)),
        None => "n/a".into(),
    }
}
