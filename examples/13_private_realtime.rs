//! Subscribe to private orders and balances concurrently.

use polyester_examples::{client_from_env, load_settings, wait_for_catalogs};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = load_settings();
    let client = client_from_env(true)?;
    wait_for_catalogs(&client).await?;

    let account_id = client
        .default_account_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("POLYESTER_ACCOUNT_ID is required for private realtime"))?;
    println!(
        "Subscribing to private orders + balances for {account_id} \
         (up to {} events each, 30s timeout)",
        settings.stream_count
    );

    let mut orders_sub = client.orders.subscribe(Some(account_id)).await?;
    let mut balances_sub = client.balances.subscribe(Some(account_id)).await?;

    let stream_count = settings.stream_count;
    let orders = tokio::spawn(async move {
        let mut seen = 0usize;
        while seen < stream_count {
            match tokio::time::timeout(Duration::from_secs(30), orders_sub.recv()).await {
                Ok(Some(order)) => {
                    println!(
                        "[orders] client_order_id={} status={} order_id={} side={}",
                        order.client_order_id, order.status, order.order_id, order.side
                    );
                    seen += 1;
                }
                Ok(None) => {
                    println!("Orders stream closed");
                    break;
                }
                Err(_) => {
                    println!("Orders stream: {seen} event(s) within 30s");
                    break;
                }
            }
        }
        orders_sub.close();
    });

    let balances = tokio::spawn(async move {
        let mut seen = 0usize;
        while seen < stream_count {
            match tokio::time::timeout(Duration::from_secs(30), balances_sub.recv()).await {
                Ok(Some(balance)) => {
                    println!(
                        "[balances] asset_id={} available={} trading={} funding={}",
                        balance.asset_id, balance.available, balance.trading, balance.funding
                    );
                    seen += 1;
                }
                Ok(None) => {
                    println!("Balances stream closed");
                    break;
                }
                Err(_) => {
                    println!("Balances stream: {seen} event(s) within 30s");
                    break;
                }
            }
        }
        balances_sub.close();
    });

    let _ = tokio::join!(orders, balances);
    println!("Private realtime example finished");
    Ok(())
}
