//! Snapshot + stream order book subscription.

use polyester::services::CreateSubscriptionOptions;
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
        "Streaming {} order book updates for {symbol} at depth={}",
        settings.stream_count, settings.orderbook_depth
    );
    let mut sub = client
        .orderbook
        .create_subscription(CreateSubscriptionOptions {
            symbol: symbol.clone(),
            depth: Some(settings.orderbook_depth),
            ..Default::default()
        })
        .await?;

    let mut seen = 0usize;
    while seen < settings.stream_count {
        match tokio::time::timeout(Duration::from_secs(30), sub.updates().recv()).await {
            Ok(Some(book)) => {
                let best_bid = book
                    .bids
                    .first()
                    .and_then(|l| l.price.as_ref())
                    .map(|p| p.format())
                    .unwrap_or_else(|| "-".into());
                let best_ask = book
                    .asks
                    .first()
                    .and_then(|l| l.price.as_ref())
                    .map(|p| p.format())
                    .unwrap_or_else(|| "-".into());
                println!(
                    "  seq={} bid={best_bid} ask={best_ask}",
                    book.book_seq
                );
                seen += 1;
            }
            Ok(None) => {
                if seen == 0 {
                    println!("Stream closed before any order book updates arrived.");
                }
                break;
            }
            Err(_) => {
                println!(
                    "No order book updates within 30s. The market may be quiet on devnet."
                );
                break;
            }
        }
    }
    sub.close();
    Ok(())
}
