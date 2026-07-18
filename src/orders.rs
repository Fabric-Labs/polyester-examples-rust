//! Order wait / cleanup helpers.

use polyester::models::{CancelOrderParams, ListOpenOrdersOpts, Order};
use polyester::Client;
use std::time::{Duration, Instant};

const OPEN_STATUSES: &[&str] = &["", "pending", "working", "pending_cancel"];
const TERMINAL_STATUSES: &[&str] = &["canceled", "rejected", "filled"];

pub fn unique_client_order_id(prefix: &str) -> String {
    let prefix = if prefix.is_empty() { "example" } else { prefix };
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{prefix}-{nanos}")
}

pub async fn wait_for_open_order(
    client: &Client,
    client_order_id: &str,
    timeout_sec: f64,
    poll_sec: f64,
) -> anyhow::Result<Order> {
    let timeout = Duration::from_secs_f64(timeout_sec.max(1.0));
    let poll = Duration::from_secs_f64(poll_sec.max(0.1));
    let deadline = Instant::now() + timeout;
    let mut last_status = String::new();

    while Instant::now() < deadline {
        if let Ok(got) = client
            .orders
            .get(Some(client_order_id), None, None)
            .await
            && let Some(order) = got.order
        {
            let status = order.status.to_ascii_lowercase();
            if OPEN_STATUSES.contains(&status.as_str()) {
                return Ok(order);
            }
            last_status = status.clone();
            if TERMINAL_STATUSES.contains(&status.as_str()) {
                return Ok(order);
            }
        }
        if let Ok(open) = client
            .orders
            .list_open_with(ListOpenOrdersOpts {
                limit: Some(50),
                ..Default::default()
            })
            .await
            && let Some(order) = open
                .orders
                .into_iter()
                .find(|o| o.client_order_id == client_order_id)
        {
            return Ok(order);
        }
        tokio::time::sleep(poll).await;
    }

    let mut msg = format!(
        "order {client_order_id} was not visible as open within {timeout_sec:.0}s"
    );
    if !last_status.is_empty() {
        msg.push_str(&format!(" (last status={last_status})"));
    }
    anyhow::bail!(msg)
}

pub async fn wait_for_no_open_order(
    client: &Client,
    client_order_id: &str,
    timeout_sec: f64,
    poll_sec: f64,
) -> anyhow::Result<()> {
    let timeout = Duration::from_secs_f64(timeout_sec.max(1.0));
    let poll = Duration::from_secs_f64(poll_sec.max(0.1));
    let deadline = Instant::now() + timeout;

    while Instant::now() < deadline {
        let still_open = match client
            .orders
            .list_open_with(ListOpenOrdersOpts {
                limit: Some(50),
                ..Default::default()
            })
            .await
        {
            Ok(open) => open
                .orders
                .iter()
                .any(|o| o.client_order_id == client_order_id),
            Err(_) => false,
        };
        if !still_open {
            return Ok(());
        }
        tokio::time::sleep(poll).await;
    }
    anyhow::bail!("order {client_order_id} still open after {timeout_sec:.0}s")
}

pub async fn cancel_all_for_symbol(client: &Client, symbol: &str) {
    let _ = client.orders.cancel_all(Some(symbol), false, None).await;
}

/// Cancel by order id when available, else by client order id.
pub async fn cancel_order(
    client: &Client,
    symbol: &str,
    order_id: &str,
    client_order_id: &str,
) -> anyhow::Result<()> {
    if !order_id.is_empty() && order_id != "0" {
        client.orders.cancel_by_order_id(order_id, None).await?;
        return Ok(());
    }
    client
        .orders
        .cancel_with(CancelOrderParams {
            order_id: None,
            client_order_id: Some(client_order_id.to_owned()),
            symbol: Some(symbol.to_owned()),
            symbol_id: None,
            subaccount_id: None,
        })
        .await?;
    Ok(())
}

pub async fn ensure_no_open_orders_with_prefix(
    client: &Client,
    prefix: &str,
) -> anyhow::Result<()> {
    let open = client
        .orders
        .list_open_with(ListOpenOrdersOpts {
            limit: Some(100),
            ..Default::default()
        })
        .await?;
    let matches: Vec<_> = open
        .orders
        .into_iter()
        .filter(|o| o.client_order_id.starts_with(prefix))
        .collect();
    if !matches.is_empty() {
        let ids = matches
            .iter()
            .map(|o| o.client_order_id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        anyhow::bail!("refusing to place a new order; open bot orders exist: {ids}");
    }
    Ok(())
}

/// Wait briefly for a fill/cancel; if still open, cancel and report the outcome.
pub async fn cancel_after_timeout(
    client: &Client,
    client_order_id: &str,
    symbol: &str,
    order_id: &str,
    timeout_sec: f64,
    poll_sec: f64,
) -> anyhow::Result<String> {
    let timeout = Duration::from_secs_f64(timeout_sec.max(1.0));
    let poll = Duration::from_secs_f64(poll_sec.max(0.1));
    let deadline = Instant::now() + timeout;

    while Instant::now() < deadline {
        if let Ok(got) = client
            .orders
            .get(Some(client_order_id), None, None)
            .await
            && let Some(order) = got.order
        {
            let status = order.status.to_ascii_lowercase();
            if TERMINAL_STATUSES.contains(&status.as_str()) {
                return Ok(status);
            }
        }
        tokio::time::sleep(poll).await;
    }

    cancel_order(client, symbol, order_id, client_order_id).await?;
    match wait_for_no_open_order(client, client_order_id, timeout_sec.max(5.0), poll_sec).await {
        Ok(()) => Ok("canceled_after_timeout".into()),
        Err(_) => Ok("canceled_after_timeout_unconfirmed".into()),
    }
}
