//! Example settings + dotenv loader (no extra runtime deps).

use rust_decimal::Decimal;
use std::env;
use std::fs;
use std::path::Path;
use std::str::FromStr;

pub const API_KEY_ID_ENV: &str = "POLYESTER_API_KEY_ID";
pub const API_PRIVATE_KEY_ENV: &str = "POLYESTER_API_PRIVATE_KEY";
pub const ACCOUNT_ID_ENV: &str = "POLYESTER_ACCOUNT_ID";
pub const SUB_ACCOUNT_ID_ENV: &str = "POLYESTER_SUB_ACCOUNT_ID";
pub const API_URL_ENV: &str = "POLYESTER_API_URL";
pub const WS_URL_ENV: &str = "POLYESTER_WS_URL";
pub const OWNER_PRIVATE_KEY_ENV: &str = "POLYESTER_OWNER_PRIVATE_KEY";

pub const EXAMPLES_SYMBOL_ENV: &str = "POLYESTER_EXAMPLES_SYMBOL";
pub const EXAMPLES_TIMEFRAME_ENV: &str = "POLYESTER_EXAMPLES_TIMEFRAME";
pub const EXAMPLES_CANDLE_LIMIT_ENV: &str = "POLYESTER_EXAMPLES_CANDLE_LIMIT";
pub const EXAMPLES_ENABLE_TRADING_ENV: &str = "POLYESTER_EXAMPLES_ENABLE_TRADING";
pub const EXAMPLES_MAX_QUOTE_ENV: &str = "POLYESTER_EXAMPLES_MAX_QUOTE";
pub const EXAMPLES_RSI_PERIOD_ENV: &str = "POLYESTER_EXAMPLES_RSI_PERIOD";
pub const EXAMPLES_RSI_OVERSOLD_ENV: &str = "POLYESTER_EXAMPLES_RSI_OVERSOLD";
pub const EXAMPLES_RSI_OVERBOUGHT_ENV: &str = "POLYESTER_EXAMPLES_RSI_OVERBOUGHT";
pub const EXAMPLES_ORDER_TIMEOUT_ENV: &str = "POLYESTER_EXAMPLES_ORDER_TIMEOUT_SEC";
pub const EXAMPLES_POLL_ENV: &str = "POLYESTER_EXAMPLES_POLL_SEC";
pub const EXAMPLES_STREAM_COUNT_ENV: &str = "POLYESTER_EXAMPLES_STREAM_COUNT";
pub const EXAMPLES_ORDERBOOK_DEPTH_ENV: &str = "POLYESTER_EXAMPLES_ORDERBOOK_DEPTH";
pub const EXAMPLES_ENABLE_TRANSFERS_ENV: &str = "POLYESTER_EXAMPLES_ENABLE_TRANSFERS";
pub const EXAMPLES_ENABLE_WITHDRAWALS_ENV: &str = "POLYESTER_EXAMPLES_ENABLE_WITHDRAWALS";
pub const EXAMPLES_ENABLE_CHAIN_FUNDING_TO_TRADING_ENV: &str =
    "POLYESTER_EXAMPLES_ENABLE_CHAIN_FUNDING_TO_TRADING";
pub const EXAMPLES_ENABLE_CHAIN_EXTERNAL_SUBMIT_ENV: &str =
    "POLYESTER_EXAMPLES_ENABLE_CHAIN_EXTERNAL_SUBMIT";
pub const EXAMPLES_TRANSFER_DEST_ACCOUNT_ID_ENV: &str =
    "POLYESTER_EXAMPLES_TRANSFER_DEST_ACCOUNT_ID";
pub const EXAMPLES_TRANSFER_AMOUNT_ENV: &str = "POLYESTER_EXAMPLES_TRANSFER_AMOUNT";
pub const EXAMPLES_WITHDRAW_AMOUNT_ENV: &str = "POLYESTER_EXAMPLES_WITHDRAW_AMOUNT";
pub const EXAMPLES_CHAIN_AMOUNT_ENV: &str = "POLYESTER_EXAMPLES_CHAIN_AMOUNT";
pub const EXAMPLES_EXTERNAL_DESTINATION_ENV: &str = "POLYESTER_EXAMPLES_EXTERNAL_DESTINATION";
pub const EXAMPLES_EXTERNAL_CHAIN_ID_ENV: &str = "POLYESTER_EXAMPLES_EXTERNAL_CHAIN_ID";

#[derive(Debug, Clone)]
pub struct ExampleSettings {
    pub symbol: String,
    pub timeframe: String,
    pub candle_limit: usize,
    pub enable_trading: bool,
    pub max_quote: Decimal,
    pub rsi_period: usize,
    pub rsi_oversold: Decimal,
    pub rsi_overbought: Decimal,
    pub order_timeout_sec: f64,
    pub poll_sec: f64,
    pub stream_count: usize,
    pub orderbook_depth: u32,
    pub enable_transfers: bool,
    pub enable_withdrawals: bool,
    pub enable_chain_funding_to_trading: bool,
    pub enable_chain_external_submit: bool,
    pub transfer_dest_account_id: String,
    pub transfer_amount: Decimal,
    pub withdraw_amount: Decimal,
    pub chain_amount: Decimal,
    pub external_destination: String,
    pub external_chain_id: u32,
    pub owner_private_key: String,
}

/// Load a simple `KEY=VALUE` `.env` without overriding already-set vars.
pub fn load_dotenv(path: impl AsRef<Path>) {
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || !line.contains('=') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim().trim_matches(|c| c == '"' || c == '\'');
        if key.is_empty() {
            continue;
        }
        if env::var_os(key).is_none() {
            // SAFETY: examples process only; no concurrent env mutation expected.
            unsafe { env::set_var(key, value) };
        }
    }
}

pub fn load_settings() -> ExampleSettings {
    load_dotenv(".env");
    ExampleSettings {
        symbol: env_string(EXAMPLES_SYMBOL_ENV, "BTC-USDT"),
        timeframe: env_string(EXAMPLES_TIMEFRAME_ENV, "1m"),
        candle_limit: env_usize(EXAMPLES_CANDLE_LIMIT_ENV, 100),
        enable_trading: env_bool(EXAMPLES_ENABLE_TRADING_ENV),
        max_quote: env_decimal(EXAMPLES_MAX_QUOTE_ENV, Decimal::from(10)),
        rsi_period: env_usize(EXAMPLES_RSI_PERIOD_ENV, 14),
        rsi_oversold: env_decimal(EXAMPLES_RSI_OVERSOLD_ENV, Decimal::from(30)),
        rsi_overbought: env_decimal(EXAMPLES_RSI_OVERBOUGHT_ENV, Decimal::from(70)),
        order_timeout_sec: env_f64(EXAMPLES_ORDER_TIMEOUT_ENV, 15.0),
        poll_sec: env_f64(EXAMPLES_POLL_ENV, 0.5),
        stream_count: env_usize(EXAMPLES_STREAM_COUNT_ENV, 5),
        orderbook_depth: env_u32(EXAMPLES_ORDERBOOK_DEPTH_ENV, 50),
        enable_transfers: env_bool(EXAMPLES_ENABLE_TRANSFERS_ENV),
        enable_withdrawals: env_bool(EXAMPLES_ENABLE_WITHDRAWALS_ENV),
        enable_chain_funding_to_trading: env_bool(EXAMPLES_ENABLE_CHAIN_FUNDING_TO_TRADING_ENV),
        enable_chain_external_submit: env_bool(EXAMPLES_ENABLE_CHAIN_EXTERNAL_SUBMIT_ENV),
        transfer_dest_account_id: env_string(EXAMPLES_TRANSFER_DEST_ACCOUNT_ID_ENV, ""),
        transfer_amount: env_decimal(EXAMPLES_TRANSFER_AMOUNT_ENV, Decimal::new(1, 2)),
        withdraw_amount: env_decimal(EXAMPLES_WITHDRAW_AMOUNT_ENV, Decimal::new(1, 2)),
        chain_amount: env_decimal(EXAMPLES_CHAIN_AMOUNT_ENV, Decimal::from(1)),
        external_destination: env_string(EXAMPLES_EXTERNAL_DESTINATION_ENV, ""),
        external_chain_id: env_u32(EXAMPLES_EXTERNAL_CHAIN_ID_ENV, 6),
        owner_private_key: env_string(OWNER_PRIVATE_KEY_ENV, ""),
    }
}

pub fn require_trading_enabled(settings: &ExampleSettings) -> anyhow::Result<()> {
    if settings.enable_trading {
        return Ok(());
    }
    anyhow::bail!(
        "live order writes are disabled. Set {EXAMPLES_ENABLE_TRADING_ENV}=1 to opt in \
         (devnet API key + trading balance required)."
    )
}

pub fn require_transfers_enabled(settings: &ExampleSettings) -> anyhow::Result<()> {
    if !settings.enable_transfers {
        anyhow::bail!(
            "internal transfers are disabled; set {EXAMPLES_ENABLE_TRANSFERS_ENV}=1 and \
             {EXAMPLES_TRANSFER_DEST_ACCOUNT_ID_ENV} after confirming destination and balances"
        );
    }
    if settings.transfer_dest_account_id.trim().is_empty()
        || looks_like_placeholder_credential(&settings.transfer_dest_account_id)
    {
        anyhow::bail!(
            "{EXAMPLES_TRANSFER_DEST_ACCOUNT_ID_ENV} is required when transfers are enabled"
        );
    }
    Ok(())
}

pub fn require_withdrawals_enabled(settings: &ExampleSettings) -> anyhow::Result<()> {
    if settings.enable_withdrawals {
        return Ok(());
    }
    anyhow::bail!(
        "withdraw submit is disabled; set {EXAMPLES_ENABLE_WITHDRAWALS_ENV}=1 after confirming \
         trading balance and intent"
    )
}

pub fn require_chain_funding_to_trading_enabled(settings: &ExampleSettings) -> anyhow::Result<()> {
    if settings.enable_chain_funding_to_trading {
        return Ok(());
    }
    anyhow::bail!(
        "chain Funding->Trading submit is disabled; set \
         {EXAMPLES_ENABLE_CHAIN_FUNDING_TO_TRADING_ENV}=1 and {OWNER_PRIVATE_KEY_ENV} to broadcast"
    )
}

pub fn require_owner_private_key(settings: &ExampleSettings) -> anyhow::Result<()> {
    if settings.owner_private_key.trim().is_empty()
        || looks_like_placeholder_credential(&settings.owner_private_key)
    {
        anyhow::bail!("{OWNER_PRIVATE_KEY_ENV} is required for smart-account UserOp submission");
    }
    Ok(())
}

pub fn looks_like_placeholder_credential(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    !lower.is_empty()
        && (lower.contains("your_")
            || lower.contains("_here")
            || lower.contains("replace_me")
            || lower.contains("changeme"))
}

fn env_string(key: &str, default: &str) -> String {
    env::var(key)
        .ok()
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

fn env_bool(key: &str) -> bool {
    matches!(
        env::var(key)
            .ok()
            .map(|v| v.trim().to_ascii_lowercase())
            .as_deref(),
        Some("1" | "true" | "yes" | "on")
    )
}

fn env_f64(key: &str, default: f64) -> f64 {
    env::var(key)
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(default)
}

fn env_usize(key: &str, default: usize) -> usize {
    env::var(key)
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(default)
}

fn env_u32(key: &str, default: u32) -> u32 {
    env::var(key)
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(default)
}

fn env_decimal(key: &str, default: Decimal) -> Decimal {
    env::var(key)
        .ok()
        .and_then(|v| Decimal::from_str(v.trim()).ok())
        .unwrap_or(default)
}
