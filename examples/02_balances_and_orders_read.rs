//! Authenticated reads: balances, open orders, history.

use polyester::models::ListOrderHistoryOpts;
use polyester::proto::ledger::read::v1::GetBalancesRequest;
use polyester_examples::{client_from_env, wait_for_catalogs};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = client_from_env(true)?;
    wait_for_catalogs(&client).await?;

    let me = client.auth.me().await?;
    println!(
        "Authenticated as account_id={} username={}",
        me.account_id,
        me.username.as_deref().unwrap_or("-")
    );

    let balances = client.balances.list(GetBalancesRequest::default()).await?;
    println!("\nBalances ({})", balances.balances.len());
    for row in balances.balances.iter().take(10) {
        println!(
            "  asset_id={} trading={} funding={} available={}",
            row.asset_id, row.trading, row.funding, row.available
        );
    }

    let open = client.orders.list_open(None).await?;
    println!("\nOpen orders ({})", open.orders.len());
    for order in open.orders.iter().take(10) {
        let price = order
            .price
            .as_ref()
            .map(|p| p.as_ticks().to_string())
            .unwrap_or_else(|| "-".into());
        println!(
            "  {} {} {} status={} price.ticks={}",
            order.order_id, order.side, order.client_order_id, order.status, price
        );
    }

    let history = client
        .orders
        .list_history_with(ListOrderHistoryOpts {
            limit: Some(10),
            ..Default::default()
        })
        .await;
    match history {
        Ok(hist) => {
            println!("\nOrder history ({})", hist.orders.len());
            for order in hist.orders.iter().take(10) {
                println!(
                    "  {} {} status={}",
                    order.order_id, order.client_order_id, order.status
                );
            }
        }
        Err(err) => {
            println!("\nOrder history unavailable (known backend flake on some envs): {err}");
        }
    }

    Ok(())
}
