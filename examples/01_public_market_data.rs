//! Public REST market data: overview, trades, candles.

use polyester_examples::{client_from_env, load_settings, pick_symbol, wait_for_catalogs};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = load_settings();
    let client = client_from_env(false)?;
    wait_for_catalogs(&client).await?;

    let overview = client.market_overview.list(Some(5)).await?;
    println!("Markets");
    for market in &overview.markets {
        let last = market
            .last_price
            .as_ref()
            .map(|p| p.format())
            .unwrap_or_else(|| "-".into());
        let index = market
            .index_price
            .as_ref()
            .map(|p| p.format())
            .unwrap_or_else(|| "-".into());
        println!(
            "  {}: symbol_id={} last_price={last} index_price={index}",
            market.symbol, market.symbol_id
        );
    }

    let spot = client.market_data.get_spot_config().await?;
    let symbol = pick_symbol(&spot, &settings.symbol);
    println!("\nUsing symbol: {symbol}");

    let trades = client.market_data.get_trades(&symbol, Some(5)).await?;
    println!("\nRecent trades");
    for trade in &trades.trades {
        let ticks = trade.price.as_ref().map(|p| p.as_ticks()).unwrap_or(0);
        let scaled = trade.qty.as_ref().map(|q| q.as_scaled()).unwrap_or(0);
        println!(
            "  {} price.ticks={} qty.scaled={}",
            trade.side, ticks, scaled
        );
    }

    let candles = client
        .market_data
        .get_candles(&symbol, &settings.timeframe, Some(5))
        .await?;
    let timeframe = if candles.timeframe.is_empty() {
        settings.timeframe.as_str()
    } else {
        candles.timeframe.as_str()
    };
    println!("\nRecent {timeframe} candles");
    for candle in &candles.candles {
        println!(
            "  ts={} open={} high={} low={} close={} volume={}",
            candle.ts_sec, candle.open, candle.high, candle.low, candle.close, candle.volume
        );
    }

    Ok(())
}
