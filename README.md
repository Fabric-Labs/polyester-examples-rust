# Polyester Rust Examples

Runnable examples for the official Polyester Rust SDK.

These examples are intentionally small. Start with read-only market data, then move to
authenticated reads, then opt in to live devnet order writes when your API key and trading
balance are ready.

## Requirements

- Rust 1.88+
- A Polyester API key for authenticated examples
- Trading balance for order-writing examples

## Install

```bash
cargo fetch
```

This repository uses a local path dependency on `../polyester-sdk-rust` when working inside the
Fabric monorepo. Point `Cargo.toml` at a git tag (`v0.1.0a21`) when consuming a published SDK
(crate is not on crates.io yet).

## Configure

```bash
cp .env.example .env
```

Fill in:

- `POLYESTER_API_KEY_ID`
- `POLYESTER_API_PRIVATE_KEY`
- `POLYESTER_ACCOUNT_ID`

If you already have `polyester-sdk-rust/.env` configured for devnet, you can reuse the same
values here. Public market-data examples can run without credentials.

The SDK does not implicitly read a `.env` file. These examples load `.env`, then pass
credentials explicitly to the client constructor.

## Safety Model

Live order writes are disabled by default. Any example that places orders requires:

```bash
POLYESTER_EXAMPLES_ENABLE_TRADING=1
```

The default max quote notional is:

```bash
POLYESTER_EXAMPLES_MAX_QUOTE=10
```

Use a devnet API key with a policy that allows trading. These examples are educational, not
production trading systems.

## Qty / price dual path

Examples use **decimal strings** for the human-readable path:

```rust
Quantity::from_decimal_str("0.01", scale, Some(symbol), None)?;
Price::from_decimal_str("100.5", Some(symbol))?;
```

For bots already in wire units, prefer scaled inputs (see `03b_scaled_int_limit_order`):

```rust
Quantity::from_scaled(qty_scaled, Some(scale), QuantityDomain::OrderBase, Some(symbol), None)?;
Price::from_ticks(price_ticks, Some(symbol))?;
```

**Anti-patterns:** do not pass `f64` / `f32`; do not pass bare integers for qty/price.

`Price` ticks are protocol units (1e6), not market tick-size alignment.
Transfers/withdraws use `AssetAmount`, not order `Quantity`.

## Funding vs Trading

Deposits land in the Funding account. Spot orders spend Trading balance.

Before running live order examples, move funds from Funding to Unified Trading in the Polyester UI
or through the wallet/on-chain flow.

## Examples

| Command | Credentials | Live orders | What it teaches |
| --- | --- | --- | --- |
| `01_public_market_data` | Optional | No | REST overview, trades, candles |
| `02_balances_and_orders_read` | Required | No | Balances, open orders, history |
| `03_place_and_cancel_limit_order` | Required | Yes (`POLYESTER_EXAMPLES_ENABLE_TRADING=1`) | Decimal qty/price create + cancel |
| `03b_scaled_int_limit_order` | Required | Yes (`POLYESTER_EXAMPLES_ENABLE_TRADING=1`) | Integer-native bot create + cancel |
| `04_public_realtime_trades` | Optional | No | Public trade websocket |
| `05_public_orderbook_stream` | Optional | No | Snapshot + stream order book |
| `06_market_overview_stream` | Optional | No | Snapshot + stream market overview |
| `07_batch_create_and_cancel_all` | Required | Yes (`POLYESTER_EXAMPLES_ENABLE_TRADING=1`) | Batch limit create, `cancel_all` |
| `08_batch_replace` | Required | Yes (`POLYESTER_EXAMPLES_ENABLE_TRADING=1`) | Batch create, replacement receipt/retry, settlement polling, successor cleanup |
| `10_rsi_signal_bot` | Optional* | Optional (`POLYESTER_EXAMPLES_ENABLE_TRADING=1`) | Candles + RSI; optional small limit |

\* Dry-run RSI needs no credentials; live mode requires auth.

```bash
cargo run --example 01_public_market_data
cargo run --example 04_public_realtime_trades
cargo run --example 05_public_orderbook_stream
cargo run --example 06_market_overview_stream
cargo run --example 02_balances_and_orders_read
cargo run --example 10_rsi_signal_bot
POLYESTER_EXAMPLES_ENABLE_TRADING=1 cargo run --example 03_place_and_cancel_limit_order
POLYESTER_EXAMPLES_ENABLE_TRADING=1 cargo run --example 03b_scaled_int_limit_order
POLYESTER_EXAMPLES_ENABLE_TRADING=1 cargo run --example 07_batch_create_and_cancel_all
POLYESTER_EXAMPLES_ENABLE_TRADING=1 cargo run --example 08_batch_replace
POLYESTER_EXAMPLES_ENABLE_TRADING=1 cargo run --example 10_rsi_signal_bot
```

Suggested order: `01` → `04`/`05`/`06` → `02` → `10` (dry) → `03` / `03b` → `07` / `10` (live) when ready.

`08_batch_replace` creates two post-only buys, submits `batch_replace` with successor client IDs,
and prints the admission receipt. The predecessor IDs are stale after admission; later tracking and
cleanup use each `replacement_order_id` / successor client ID. It shows how to retry an ambiguous
call with the same `request_id`, then polls `get_batch_replace_status` until
`is_batch_replace_settled` reports every item is `working`, `rejected`, or `terminal`.

TWAP, ladder, and standalone trigger examples are intentionally omitted for v1. They use the
triggers API (separate lifecycle from normal orders) and are a poor fit for a small cookbook.

Realtime examples exit after 30 seconds if no data arrives (common on quiet
devnet markets).

## Useful Settings

- `POLYESTER_EXAMPLES_SYMBOL`: default `BTC-USDT`
- `POLYESTER_EXAMPLES_TIMEFRAME`: default `1m`
- `POLYESTER_EXAMPLES_CANDLE_LIMIT`: default `100`
- `POLYESTER_EXAMPLES_MAX_QUOTE`: default `10`
- `POLYESTER_EXAMPLES_RSI_PERIOD`: default `14`
- `POLYESTER_EXAMPLES_RSI_OVERSOLD`: default `30`
- `POLYESTER_EXAMPLES_RSI_OVERBOUGHT`: default `70`
- `POLYESTER_EXAMPLES_ORDER_TIMEOUT_SEC`: default `15`
- `POLYESTER_EXAMPLES_STREAM_COUNT`: default `5`
- `POLYESTER_EXAMPLES_ORDERBOOK_DEPTH`: default `50`

## Notes For Bot Builders

- Pass `Price` / `Quantity` wrappers only. Do not use floats or bare integers for order inputs.
- Prefer decimal constructors for readable demos; prefer `from_ticks` / `from_scaled` when your bot
  already works in wire units.
- Use client order IDs for idempotency and cleanup.
- Check open orders before placing a new bot order.
- Decide how your production bot will track positions. The RSI example intentionally avoids a
  persistent state file and only uses balances plus open orders.
- Treat the RSI strategy as a teaching example. It is deliberately naive.

## Development

```bash
cargo test
cargo check --examples
```
