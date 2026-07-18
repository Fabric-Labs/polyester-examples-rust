//! Snapshot + stream market overview subscription.

use polyester::models::MarketOverviewEntry;
use polyester::services::MarketOverviewCreateSubscriptionOptions;
use polyester_examples::{client_from_env, load_settings, pick_symbol, wait_for_catalogs};
use std::time::Duration;

fn row_for_symbol<'a>(
    rows: &'a [MarketOverviewEntry],
    symbol: &str,
) -> Option<&'a MarketOverviewEntry> {
    rows.iter()
        .find(|row| row.symbol == symbol)
        .or_else(|| rows.first())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = load_settings();
    let client = client_from_env(false)?;
    wait_for_catalogs(&client).await?;

    let spot = client.market_data.get_spot_config().await?;
    let symbol = pick_symbol(&spot, &settings.symbol);

    println!(
        "Streaming {} merged market-overview snapshots (highlighting {symbol})",
        settings.stream_count
    );
    let mut sub = client
        .market_overview
        .create_subscription(MarketOverviewCreateSubscriptionOptions {
            symbols: Some(vec![symbol.clone()]),
            limit: Some(10),
            ..Default::default()
        })
        .await?;

    let mut seen = 0usize;
    while seen < settings.stream_count {
        match tokio::time::timeout(Duration::from_secs(30), sub.updates().recv()).await {
            Ok(Some(rows)) => {
                let focus = row_for_symbol(&rows, &symbol);
                let label = focus.map(|r| r.symbol.as_str()).unwrap_or(symbol.as_str());
                let price = focus
                    .and_then(|r| r.last_price.as_ref())
                    .map(|p| p.format())
                    .unwrap_or_else(|| "-".into());
                println!(
                    "  update={} rows={} {label} last_price={price}",
                    seen + 1,
                    rows.len()
                );
                seen += 1;
            }
            Ok(None) => {
                if seen == 0 {
                    println!("Stream closed before any overview snapshots arrived.");
                }
                break;
            }
            Err(_) => {
                println!("No overview updates within 30s. The market may be quiet on devnet.");
                break;
            }
        }
    }
    sub.close();
    Ok(())
}
