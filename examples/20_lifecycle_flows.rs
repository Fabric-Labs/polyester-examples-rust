//! List lifecycle flows and print precise lifecycle / Zipper failure reasons.

use polyester::proto::chain::lifecycle::v1::ListFlowsRequest;
use polyester::Error;
use polyester_examples::{client_from_env, load_settings};

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

    Ok(())
}
