//! Public trade websocket stream.

use polyester_examples::{client_from_env, load_settings, pick_symbol, wait_for_catalogs};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = load_settings();
    let client = client_from_env(false)?;
    wait_for_catalogs(&client).await?;

    let spot = client.market_data.get_spot_config().await?;
    let symbol = pick_symbol(&spot, &settings.symbol);

    println!(
        "Streaming {} public trades for {symbol}",
        settings.stream_count
    );
    let mut sub = client.market_data.subscribe_trades(&symbol).await?;
    let mut seen = 0usize;
    while seen < settings.stream_count {
        match tokio::time::timeout(Duration::from_secs(30), sub.recv()).await {
            Ok(Some(trade)) => {
                let ticks = trade.price.as_ref().map(|p| p.as_ticks()).unwrap_or(0);
                let scaled = trade.qty.as_ref().map(|q| q.as_scaled()).unwrap_or(0);
                println!(
                    "  {} price.ticks={ticks} qty.scaled={scaled} match_id={}",
                    trade.side, trade.match_id
                );
                seen += 1;
            }
            Ok(None) => {
                if seen == 0 {
                    println!("Stream closed before any trades arrived.");
                }
                break;
            }
            Err(_) => {
                println!("No trades received within 30s. The market may be quiet on devnet.");
                break;
            }
        }
    }
    sub.close();
    Ok(())
}
