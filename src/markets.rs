//! Symbol / sizing helpers for trading examples.

use polyester::codecs::scalars::{format_price_ticks, parse_price_ticks_str};
use polyester::models::{AssetBalance, OrderbookData, SpotConfig};
use polyester::proto::marketdata::v1::{GetSpotConfigResponse, PairConfig};
use polyester::Client;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::str::FromStr;

const CANDIDATES: &[&str] = &["BTC-USDT", "ETH-USDT", "SOL-USDT", "BNB-USDT"];

fn spot_proto(spot: &SpotConfig) -> GetSpotConfigResponse {
    serde_json::from_value(spot.raw.clone()).unwrap_or_default()
}

pub fn pick_symbol(spot: &SpotConfig, preferred: &str) -> String {
    let symbols: Vec<String> = spot_proto(spot)
        .pairs
        .into_iter()
        .map(|p| p.symbol)
        .filter(|s| !s.trim().is_empty())
        .collect();
    let preferred = preferred.trim();
    if !preferred.is_empty() && symbols.iter().any(|s| s == preferred) {
        return preferred.to_owned();
    }
    for candidate in CANDIDATES {
        if symbols.iter().any(|s| s == *candidate) {
            return (*candidate).to_owned();
        }
    }
    symbols
        .into_iter()
        .next()
        .unwrap_or_else(|| "BTC-USDT".to_owned())
}

pub fn pair_for_symbol(spot: &SpotConfig, symbol: &str) -> Option<PairConfig> {
    spot_proto(spot)
        .pairs
        .into_iter()
        .find(|p| p.symbol == symbol)
}

pub fn quantity_scale_for_pair(pair: Option<&PairConfig>) -> u32 {
    pair.map(|p| {
        if p.base_quantity_scale > 0 {
            p.base_quantity_scale
        } else {
            8
        }
    })
    .unwrap_or(8)
}

pub fn quote_asset_id(client: &Client, pair: Option<&PairConfig>, symbol: &str) -> Option<u32> {
    if let Some(pair) = pair
        && !pair.quote_asset.is_empty()
        && let Some(id) = client.catalogs.ledger_id_for_asset(&pair.quote_asset)
    {
        return Some(id);
    }
    let quote = symbol.split('-').nth(1).unwrap_or("USDT");
    client.catalogs.ledger_id_for_asset(quote)
}

pub fn base_asset_id(client: &Client, pair: Option<&PairConfig>, symbol: &str) -> Option<u32> {
    if let Some(pair) = pair
        && !pair.base_asset.is_empty()
        && let Some(id) = client.catalogs.ledger_id_for_asset(&pair.base_asset)
    {
        return Some(id);
    }
    let base = symbol.split('-').next().unwrap_or("BTC");
    client.catalogs.ledger_id_for_asset(base)
}

pub fn quote_asset_symbol(pair: Option<&PairConfig>, symbol: &str) -> String {
    if let Some(pair) = pair
        && !pair.quote_asset.is_empty()
    {
        return pair.quote_asset.clone();
    }
    symbol
        .split('-')
        .nth(1)
        .unwrap_or("USDT")
        .to_owned()
}

pub fn format_decimal(value: Decimal) -> String {
    let text = value.normalize().to_string();
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else if text.is_empty() {
        "0".into()
    } else {
        text
    }
}

fn tick_size(pair: Option<&PairConfig>) -> Decimal {
    pair.and_then(|p| {
        let s = p.tick_size.trim();
        if s.is_empty() {
            None
        } else {
            Decimal::from_str(s).ok()
        }
    })
    .unwrap_or(Decimal::new(1, 2))
}

fn align_to_step(value: Decimal, step: Decimal, round_up: bool) -> Decimal {
    if step <= Decimal::ZERO {
        return value;
    }
    let steps = value / step;
    let aligned = if round_up {
        steps.ceil()
    } else {
        steps.trunc()
    };
    aligned * step
}

/// Slightly aggressive limit price for a teaching bot (not post-only).
pub fn marketable_limit_price(
    side: &str,
    last_price: Decimal,
    pair: Option<&PairConfig>,
) -> anyhow::Result<String> {
    let step = tick_size(pair);
    match side {
        "buy" => {
            let target = last_price * Decimal::new(1001, 3);
            Ok(format_decimal(align_to_step(target, step, true)))
        }
        "sell" => {
            let target = last_price * Decimal::new(999, 3);
            Ok(format_decimal(align_to_step(target, step, false)))
        }
        _ => anyhow::bail!("side must be 'buy' or 'sell'"),
    }
}

/// Size a sell so quote notional is at most `max_quote`, respecting step/min qty.
pub fn sell_qty_for_quote_cap(
    available_base: Decimal,
    max_quote: Decimal,
    price: Decimal,
    pair: Option<&PairConfig>,
) -> anyhow::Result<String> {
    if price <= Decimal::ZERO {
        anyhow::bail!("price must be positive");
    }
    let step = pair
        .and_then(|p| {
            let s = p.step_size.trim();
            if s.is_empty() {
                None
            } else {
                Decimal::from_str(s).ok()
            }
        })
        .unwrap_or(Decimal::new(1, 3));
    let min_notional = pair
        .and_then(|p| {
            let s = p.min_notional_quote.trim();
            if s.is_empty() {
                None
            } else {
                Decimal::from_str(s).ok()
            }
        })
        .unwrap_or(Decimal::from(10));

    let mut qty = available_base.min(max_quote / price);
    if step > Decimal::ZERO {
        qty = (qty / step).trunc() * step;
    }
    if qty <= Decimal::ZERO {
        anyhow::bail!("no base trading balance is available to sell");
    }
    if qty * price < min_notional {
        anyhow::bail!(
            "sell size is below min notional {min_notional}; available_base={available_base}"
        );
    }
    Ok(qty.normalize().to_string())
}

fn ledger_amount_to_decimal(raw: &str) -> Decimal {
    let Ok(n) = raw.parse::<u128>() else {
        return Decimal::ZERO;
    };
    let s = n.to_string();
    if s.len() <= 18 {
        let frac = format!("{s:0>18}");
        format!("0.{frac}").parse().unwrap_or(Decimal::ZERO)
    } else {
        let split = s.len() - 18;
        let (whole, frac) = s.split_at(split);
        format!("{whole}.{frac}").parse().unwrap_or(Decimal::ZERO)
    }
}

pub fn available_trading_balance(balances: &[AssetBalance], asset_id: u32) -> Decimal {
    balances
        .iter()
        .find(|b| b.asset_id == asset_id)
        .map(|b| ledger_amount_to_decimal(&b.trading))
        .unwrap_or(Decimal::ZERO)
}

fn post_only_buy_price_from_book(book: &OrderbookData, tick_size: &str) -> Option<String> {
    let tick_ticks = parse_price_ticks_str(tick_size, "tick_size").ok()?;
    if tick_ticks == 0 || book.bids.is_empty() {
        return None;
    }
    let bid_ticks = book.bids[0].price.as_ref()?.as_ticks();
    let mut target = bid_ticks - tick_ticks;
    if target < tick_ticks {
        target = tick_ticks;
    }
    if let Some(ask) = book.asks.first()
        && let Some(ask_px) = ask.price.as_ref()
        && ask_px.as_ticks() > 0
    {
        let max_post_only = ask_px.as_ticks() - tick_ticks;
        if target > max_post_only {
            target = max_post_only;
        }
    }
    if target < tick_ticks {
        return None;
    }
    Some(format_price_ticks(target))
}

pub async fn resolve_post_only_buy_limit_price(
    client: &Client,
    symbol: &str,
    pair: Option<&PairConfig>,
) -> anyhow::Result<String> {
    let tick_size = pair
        .map(|p| {
            if p.tick_size.trim().is_empty() {
                "0.01"
            } else {
                p.tick_size.trim()
            }
        })
        .unwrap_or("0.01");
    if let Ok(book) = client.orderbook.get(symbol, None).await
        && let Some(price) = post_only_buy_price_from_book(&book, tick_size)
    {
        return Ok(price);
    }
    anyhow::bail!("could not resolve a post-only buy limit price for {symbol}")
}

/// Size a buy so quote notional is at most `max_quote`, respecting step/min qty.
pub fn buy_qty_for_quote_cap(
    available_quote: Decimal,
    max_quote: Decimal,
    price: Decimal,
    pair: Option<&PairConfig>,
) -> anyhow::Result<String> {
    if price <= Decimal::ZERO {
        anyhow::bail!("price must be positive");
    }
    let step = pair
        .and_then(|p| {
            let s = p.step_size.trim();
            if s.is_empty() {
                None
            } else {
                Decimal::from_str(s).ok()
            }
        })
        .unwrap_or(Decimal::new(1, 3));
    let min_qty = pair
        .and_then(|p| {
            let s = p.min_qty_base.trim();
            if s.is_empty() {
                None
            } else {
                Decimal::from_str(s).ok()
            }
        })
        .unwrap_or(step);
    let min_notional = pair
        .and_then(|p| {
            let s = p.min_notional_quote.trim();
            if s.is_empty() {
                None
            } else {
                Decimal::from_str(s).ok()
            }
        })
        .unwrap_or(Decimal::from(10));

    let budget = available_quote.min(max_quote);
    if budget < min_notional {
        anyhow::bail!(
            "available quote {available_quote} (cap {max_quote}) is below min notional {min_notional}"
        );
    }
    let mut qty = budget / price;
    if step > Decimal::ZERO {
        let steps = (qty / step).trunc();
        qty = steps * step;
    }
    if qty < min_qty {
        anyhow::bail!("sized qty {qty} is below min qty {min_qty}");
    }
    // Trim trailing zeros for a clean decimal string.
    Ok(qty.normalize().to_string())
}

/// Convert a human decimal qty into wire scaled units for the bot path demo.
pub fn qty_to_scaled(qty_decimal: &str, scale: u32) -> anyhow::Result<i64> {
    let qty = Decimal::from_str(qty_decimal.trim())?;
    let mult = Decimal::from(10u64.pow(scale));
    let scaled = qty * mult;
    if scaled != scaled.trunc() {
        anyhow::bail!("qty {qty_decimal:?} does not scale cleanly to {scale} decimals");
    }
    scaled
        .to_i64()
        .ok_or_else(|| anyhow::anyhow!("scaled qty out of i64 range"))
}

/// Convert a human decimal price into price ticks (1e6).
pub fn price_to_ticks(price_decimal: &str) -> anyhow::Result<i64> {
    Ok(parse_price_ticks_str(price_decimal, "price")?)
}
