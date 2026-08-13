//! Read VIP catalog/status, effective spot fees, and trading rate limits.

use polyester::Error;
use polyester_examples::{client_from_env, load_settings};

fn unavailable(err: &Error) -> bool {
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

    match client.vip.list_vip_tiers().await {
        Ok(tiers) => {
            println!(
                "VIP catalog policy_version={} tiers={} retention_bp={}",
                tiers.policy_version,
                tiers.tiers.len(),
                tiers.retention_threshold_bp
            );
            for row in &tiers.tiers {
                let aop = row.aop_threshold_usd.as_deref().unwrap_or("omitted");
                println!(
                    "  VIP{} volume_usd={} aop_usd={} maker={}% taker={}%",
                    row.tier,
                    row.volume_threshold_usd,
                    aop,
                    row.maker_fee_rate_percent,
                    row.taker_fee_rate_percent
                );
            }
        }
        Err(err) if unavailable(&err) => {
            println!("VIP catalog unavailable on this host: {err}");
        }
        Err(err) => return Err(err.into()),
    }

    match client.vip.get_vip_status().await {
        Ok(status) => {
            println!(
                "VIP status tier={} volume_tier={} aop_tier={} volume_30d={:?} aop_30d={:?}",
                status.tier,
                status.volume_tier,
                status.aop_tier,
                status.settled_volume_30d_usd,
                status.average_aop_30d_usd
            );
            if let Some(nxt) = &status.next_tier_thresholds {
                println!(
                    "  next VIP{} volume_usd={} aop_usd={}",
                    nxt.tier, nxt.volume_threshold_usd, nxt.aop_threshold_usd
                );
            }
        }
        Err(err) if unavailable(&err) => {
            println!("VIP status unavailable on this host: {err}");
        }
        Err(err) => return Err(err.into()),
    }

    match client.fees.get_spot_fee_rates(None, vec![]).await {
        Ok(fees) => {
            println!("Spot fee rates ({})", fees.fee_rates.len());
            for row in fees.fee_rates.iter().take(8) {
                println!(
                    "  {} maker={}% taker={}% vip={}",
                    row.symbol,
                    row.maker_fee_rate_percent,
                    row.taker_fee_rate_percent,
                    row.vip_tier
                );
            }
            if fees.fee_rates.len() > 8 {
                println!("  ... {} more", fees.fee_rates.len() - 8);
            }
        }
        Err(err) if unavailable(&err) => {
            println!("spot fee rates unavailable on this host: {err}");
        }
        Err(err) => return Err(err.into()),
    }

    match client.rate_limits.get_rate_limit_config().await {
        Ok(catalog) => {
            println!(
                "Rate-limit catalog policy_version={} rules={}",
                catalog.policy_version,
                catalog.rules.len()
            );
            for rule in catalog.rules.iter().take(6) {
                println!(
                    "  {} VIP{} quota={}/{}ms burst={}",
                    rule.policy_class,
                    rule.tier,
                    rule.quota_weight,
                    rule.period_ms,
                    rule.burst_weight
                );
            }
        }
        Err(err) if unavailable(&err) => {
            println!("rate-limit catalog unavailable on this host: {err}");
        }
        Err(err) => return Err(err.into()),
    }

    match client.rate_limits.get_trading_rate_limits(None).await {
        Ok(limits) => {
            println!(
                "Trading rate limits policy_version={} rules={} api_key_rules={}",
                limits.policy_version,
                limits.rules.len(),
                limits.api_key_rules.len()
            );
        }
        Err(err) if unavailable(&err) => {
            println!("trading rate limits unavailable on this host: {err}");
        }
        Err(err) => return Err(err.into()),
    }

    Ok(())
}
