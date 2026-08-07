//! List lifecycle flows and print precise lifecycle / Zipper failure reasons.

use polyester::Error;
use polyester::proto::chain::lifecycle::v1::{
    ListFlowsByTxRequest, ListFlowsRequest, TxLookupKind,
};
use polyester_examples::{client_from_env, load_settings};

const LIFECYCLE_TX_HASH_ENV: &str = "POLYESTER_EXAMPLES_LIFECYCLE_TX_HASH";

fn lifecycle_unavailable(err: &Error) -> bool {
    match err {
        Error::RouteNotFound { .. } => true,
        Error::Api { code, .. } => matches!(code.as_str(), "unimplemented" | "not_found"),
        _ => false,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _settings = load_settings();
    let client = client_from_env(true)?;

    let flows = match client
        .lifecycle
        .list_flows(ListFlowsRequest {
            limit: 20,
            ..Default::default()
        })
        .await
    {
        Ok(flows) => flows,
        Err(err) if lifecycle_unavailable(&err) => {
            println!("Lifecycle list unavailable on this host: {err}");
            return Ok(());
        }
        Err(err) => return Err(err.into()),
    };

    println!("Lifecycle flows ({})", flows.flows.len());
    for flow in &flows.flows {
        println!(
            "  intent_id={} kind={} step={} open={} terminal={} reason={}",
            flow.intent_id,
            flow.flow_kind,
            flow.latest_step,
            flow.is_open,
            flow.is_terminal,
            flow.lifecycle_reason
        );
        if let Some(zipper) = &flow.zipper_reason {
            println!(
                "    zipper_reason reason_id={:?} message={:?} code={}",
                zipper.reason_id, zipper.message, zipper.code
            );
        }
    }

    let tx_hash = std::env::var(LIFECYCLE_TX_HASH_ENV)
        .unwrap_or_default()
        .trim()
        .to_owned();
    if tx_hash.is_empty() {
        println!("Set {LIFECYCLE_TX_HASH_ENV} to demonstrate paginated transaction lookup.");
        return Ok(());
    }

    let mut request = ListFlowsByTxRequest {
        tx_hash: tx_hash.clone(),
        lookup_kind: TxLookupKind::TX_ANY.into(),
        limit: 50,
        ..Default::default()
    };
    let mut matches = Vec::new();
    loop {
        let page = client.lifecycle.list_flows_by_tx(request.clone()).await?;
        matches.extend(page.flows.into_iter().map(|flow| flow.intent_id));
        if page.next_page_token.is_empty() {
            break;
        }
        request.page_token = page.next_page_token;
    }

    println!("Transaction matches ({}) for {tx_hash}", matches.len());
    for flow_id in matches {
        println!("  intent_id={flow_id}");
    }

    Ok(())
}
