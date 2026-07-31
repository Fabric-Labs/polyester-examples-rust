//! Shared helpers for Polyester Rust SDK examples.

pub mod client;
pub mod config;
pub mod indicators;
pub mod markets;
pub mod orders;

pub use client::{client_from_env, wait_for_catalogs};
pub use config::{
    EXAMPLES_ENABLE_CHAIN_EXTERNAL_SUBMIT_ENV, EXAMPLES_ENABLE_CHAIN_FUNDING_TO_TRADING_ENV,
    EXAMPLES_ENABLE_TRANSFERS_ENV, EXAMPLES_ENABLE_WITHDRAWALS_ENV,
    EXAMPLES_EXTERNAL_DESTINATION_ENV, EXAMPLES_TRANSFER_DEST_ACCOUNT_ID_ENV, ExampleSettings,
    OWNER_PRIVATE_KEY_ENV, load_dotenv, load_settings, require_chain_funding_to_trading_enabled,
    require_owner_private_key, require_trading_enabled, require_transfers_enabled,
    require_withdrawals_enabled,
};
pub use indicators::{RsiSignal, calculate_rsi, rsi_signal};
pub use markets::{
    available_trading_balance, base_asset_id, buy_qty_for_quote_cap, format_decimal,
    human_amount_to_e18, marketable_limit_price, min_base_qty_for_pair, pair_for_symbol,
    pick_symbol, pick_usdt_zipper_asset, price_to_ticks, qty_to_scaled, quantity_scale_for_pair,
    quantity_scale_for_symbol, quote_asset_id, quote_asset_symbol,
    resolve_post_only_buy_limit_price, sell_qty_for_quote_cap, slightly_lower_limit_price,
    tick_size,
};
pub use orders::{
    cancel_after_timeout, cancel_all_for_symbol, cancel_order, cancel_owned_orders_with_prefix,
    ensure_no_open_orders_with_prefix, unique_client_order_id, wait_for_no_open_order,
    wait_for_open_order,
};
