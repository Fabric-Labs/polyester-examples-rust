//! Shared helpers for Polyester Rust SDK examples.

pub mod client;
pub mod config;
pub mod indicators;
pub mod markets;
pub mod orders;

pub use client::{client_from_env, wait_for_catalogs};
pub use config::{ExampleSettings, load_dotenv, load_settings, require_trading_enabled};
pub use indicators::{RsiSignal, calculate_rsi, rsi_signal};
pub use markets::{
    available_trading_balance, base_asset_id, buy_qty_for_quote_cap, format_decimal,
    marketable_limit_price, pair_for_symbol, pick_symbol, price_to_ticks, qty_to_scaled,
    quantity_scale_for_pair, quantity_scale_for_symbol, quote_asset_id, quote_asset_symbol,
    resolve_post_only_buy_limit_price, sell_qty_for_quote_cap, slightly_lower_limit_price,
};
pub use orders::{
    cancel_after_timeout, cancel_all_for_symbol, cancel_order, cancel_owned_orders_with_prefix,
    ensure_no_open_orders_with_prefix, unique_client_order_id, wait_for_no_open_order,
    wait_for_open_order,
};
