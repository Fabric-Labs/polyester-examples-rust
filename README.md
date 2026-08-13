# Polyester Rust Examples

Runnable examples for the official Polyester Rust SDK.

These examples are intentionally small. Start with read-only market data, then move to
authenticated reads, then opt in to live devnet order writes, transfers, withdrawals, or chain
Funding UserOps when your credentials and balances are ready.

## Requirements

- Rust 1.88+
- A Polyester API key for authenticated examples
- Trading balance for order-writing examples
- Funding balance + owner private key for chain Funding-to-Trading / Funding-to-external submit

## Install

```bash
cargo fetch
```

This repository uses a local path dependency on `../polyester-sdk-rust` when working inside the
Fabric monorepo. For a standalone clone, pin
`polyester-sdk = "0.1.0-alpha.37"` from [crates.io](https://crates.io/crates/polyester-sdk)
(or a git tag `v0.1.0a37`).

## Configure

```bash
cp .env.example .env
```

Fill in:

- `POLYESTER_API_KEY_ID`
- `POLYESTER_API_PRIVATE_KEY`
- `POLYESTER_ACCOUNT_ID`

If you already have `polyester-sdk-rust/.env` configured for devnet, you can reuse the same three
values here. Placeholder text from `.env.example` is ignored for public examples but will fail
authenticated or trading examples until replaced with real credentials. Username is optional;
API-key authentication with the Account ID is valid.

Public market-data examples can run without credentials. Authenticated reads and all order examples
require an API key.

For a subaccount-scoped key, attach an API-key policy that grants ledger read permission for balance
reads and private balance streams. Add trading permission for order mutations. The API-key policy is
distinct from the subaccount policy, and both apply.

The SDK does not implicitly read a `.env` file. These examples load `.env`, then pass credentials
explicitly to the client constructor.

Load env vars in your shell when running examples:

```bash
set -a && source .env && set +a
```

## Safety Model

Opt-in flags are separate. Never overload `POLYESTER_EXAMPLES_ENABLE_TRADING` onto transfers,
withdrawals, or chain submit:

| Flag | Gates |
| --- | --- |
| `POLYESTER_EXAMPLES_ENABLE_TRADING=1` | Order / trigger writes (`03`, `03b`, `07`-`12`, `18`, live `10`) |
| `POLYESTER_EXAMPLES_ENABLE_TRANSFERS=1` | Internal transfer submit (`14`) + dest account env |
| `POLYESTER_EXAMPLES_ENABLE_WITHDRAWALS=1` | API-key withdraw submit (`15`; prepare always runs) |
| `POLYESTER_EXAMPLES_ENABLE_CHAIN_FUNDING_TO_TRADING=1` | Funding-to-Trading UserOp submit (`16`; encode always) |
| `POLYESTER_EXAMPLES_ENABLE_CHAIN_EXTERNAL_SUBMIT=1` | Funding-to-external UserOp submit (`17`; encode always) |

The default max quote notional for order examples is:

```bash
POLYESTER_EXAMPLES_MAX_QUOTE=10
```

Use a devnet API key with a policy that allows the actions you enable. These examples are
educational, not production trading systems.

## Qty / Price Dual Path

Examples use decimal strings for human-readable order qty and price:

```rust
Quantity::from_decimal_str("0.01", scale, Some(symbol), None)?;
Price::from_decimal_str("100.5", Some(symbol))?;
```

For bots already in wire units, prefer scaled inputs:

```rust
Quantity::from_scaled(qty_scaled, Some(scale), QuantityDomain::OrderBase, Some(symbol), None)?;
Price::from_ticks(price_ticks, Some(symbol))?;
```

Do not pass `f64` / `f32`; do not pass bare integers for qty/price.

`Price` ticks are protocol units (1e6), not market tick-size alignment. Transfers/withdraws use
`AssetAmount`, not order `Quantity`. Private order-stream quantity scale metadata may be absent.
Treat raw scaled values as wire units and resolve the base quantity scale from hydrated catalogs by
symbol or `symbol_id`; never invent a fallback scale.

## Funding vs Trading

Deposits land in the Funding account. Spot orders spend Trading balance.

Before running live order examples, move funds from Funding to Unified Trading:

- In the Polyester UI / wallet flow, or
- Via example `16_funding_to_trading` (encodes `TradingGateway.deposit`; set
  `POLYESTER_EXAMPLES_ENABLE_CHAIN_FUNDING_TO_TRADING=1` plus
  `POLYESTER_OWNER_PRIVATE_KEY` to broadcast a UserOp)

API-key Trading-to-Funding withdraw is example `15` (prepare always; submit only when
`POLYESTER_EXAMPLES_ENABLE_WITHDRAWALS=1`).

## Examples

Run examples from the repository root after configuring `.env`.

| Command | Credentials | Opt-in | What it teaches |
| --- | --- | --- | --- |
| `01_public_market_data` | Optional | - | REST overview, trades, candles |
| `02_balances_and_orders_read` | Required | - | Balances, open orders, history |
| `19_preview_order` | Required | - | PreviewOrder admissibility + protected price bound |
| `20_lifecycle_flows` | Required | `LIFECYCLE_TX_HASH` (optional) | Lifecycle reasons, Zipper details, paginated transaction matches |
| `21_vip_fees_rate_limits` | Required | - | VIP catalog/status, effective spot fees, trading rate limits |
| `03_place_and_cancel_limit_order` | Required | `ENABLE_TRADING` | Decimal qty/price create + cancel |
| `03b_scaled_int_limit_order` | Required | `ENABLE_TRADING` | Integer-native bot create + cancel |
| `04_public_realtime_trades` | Optional | - | Public trade websocket |
| `05_public_orderbook_stream` | Optional | - | Snapshot + stream order book |
| `06_market_overview_stream` | Optional | - | Snapshot + stream market overview |
| `07_batch_create_and_cancel_all` | Required | `ENABLE_TRADING` | Batch limit create, `cancel_all` |
| `08_batch_replace` | Required | `ENABLE_TRADING` | Batch create, `batch_replace` price, cleanup |
| `09_batch_cancel` | Required | `ENABLE_TRADING` | Batch create, `Orders.batch_cancel` by client id |
| `10_rsi_signal_bot` | Required for live | Optional `ENABLE_TRADING` | Candles + RSI; optional small limit |
| `11_twap_trigger` | Required | `ENABLE_TRADING` | Triggers API TWAP create -> list/get -> cancel |
| `12_ladder_trigger` | Required | `ENABLE_TRADING` | Triggers API ladder create -> list/get -> cancel |
| `18_trailing_stop_trigger` | Required | `ENABLE_TRADING` | Standalone trailing stop (SELL); read type/side/parent |
| `13_private_realtime` | Required | - | Private orders + balances websocket |
| `14_internal_transfer` | Required | `ENABLE_TRANSFERS` + dest account | Tiny `InternalTransfers.create` |
| `15_api_key_trading_withdraw` | Required | Prepare always; `ENABLE_WITHDRAWALS` to submit | Trading-to-Funding prepare / submit |
| `16_funding_to_trading` | Required | Encode always; `ENABLE_CHAIN_FUNDING_TO_TRADING` to submit | Encode deposit; optional UserOp |
| `17_funding_to_external` | Required | Encode needs dest; `ENABLE_CHAIN_EXTERNAL_SUBMIT` to submit | Encode withdrawToChain; optional UserOp |

Suggested order: `01` -> `04`/`05`/`06` -> `02` -> `13` -> `10` (dry) -> `03` / `03b` ->
`07`/`08`/`09` -> `11`/`12`/`18` -> money-movement examples when those flags are intentionally enabled.

### Live Smoke

```bash
make live-smoke
# or: bash scripts/live-smoke.sh
```

Runs all examples in order. Gated examples print `SKIP` and continue when their flag is missing.
Set `LIVE_SMOKE_STRICT=1` to fail instead of skipping.

### Read-Only

```bash
cargo run --example 01_public_market_data
cargo run --example 04_public_realtime_trades
cargo run --example 05_public_orderbook_stream
cargo run --example 06_market_overview_stream
```

Realtime examples exit after 30 seconds if no data arrives (common on quiet devnet markets).

### Authenticated Reads

```bash
cargo run --example 02_balances_and_orders_read
cargo run --example 19_preview_order
cargo run --example 20_lifecycle_flows
cargo run --example 21_vip_fees_rate_limits
cargo run --example 13_private_realtime
```

Set `POLYESTER_EXAMPLES_LIFECYCLE_TX_HASH` before running example `20` to list
every lifecycle flow associated with one transaction, following all pagination
tokens.

`13` subscribes to private orders and balances concurrently and prints up to
`POLYESTER_EXAMPLES_STREAM_COUNT` events per stream (or 30s timeout). No trading flag.

### Explicit Live Writes

```bash
POLYESTER_EXAMPLES_ENABLE_TRADING=1 cargo run --example 03_place_and_cancel_limit_order
POLYESTER_EXAMPLES_ENABLE_TRADING=1 cargo run --example 03b_scaled_int_limit_order
POLYESTER_EXAMPLES_ENABLE_TRADING=1 cargo run --example 07_batch_create_and_cancel_all
POLYESTER_EXAMPLES_ENABLE_TRADING=1 cargo run --example 08_batch_replace
POLYESTER_EXAMPLES_ENABLE_TRADING=1 cargo run --example 09_batch_cancel
POLYESTER_EXAMPLES_ENABLE_TRADING=1 cargo run --example 11_twap_trigger
POLYESTER_EXAMPLES_ENABLE_TRADING=1 cargo run --example 12_ladder_trigger
POLYESTER_EXAMPLES_ENABLE_TRADING=1 cargo run --example 18_trailing_stop_trigger
```

`07` demonstrates batch create followed by account-scoped `cancel_all`. `08` demonstrates
`Orders.batch_replace`. `09` demonstrates `Orders.batch_cancel`. TWAP/ladder/trailing use the
Triggers API (separate lifecycle from normal orders): create -> list/get -> cancel, plus
best-effort `cancel_all` for resting child orders. Trailing reads print wire `side` and
`parent_order_id` (empty for standalone).

```bash
cargo run --example 10_rsi_signal_bot
POLYESTER_EXAMPLES_ENABLE_TRADING=1 cargo run --example 10_rsi_signal_bot
```

### Transfers / Withdrawals / Chain

```bash
POLYESTER_EXAMPLES_ENABLE_TRANSFERS=1 \
POLYESTER_EXAMPLES_TRANSFER_DEST_ACCOUNT_ID=... \
cargo run --example 14_internal_transfer

cargo run --example 15_api_key_trading_withdraw
POLYESTER_EXAMPLES_ENABLE_WITHDRAWALS=1 cargo run --example 15_api_key_trading_withdraw

cargo run --example 16_funding_to_trading
POLYESTER_EXAMPLES_ENABLE_CHAIN_FUNDING_TO_TRADING=1 \
POLYESTER_OWNER_PRIVATE_KEY=0x... \
cargo run --example 16_funding_to_trading

POLYESTER_EXAMPLES_EXTERNAL_DESTINATION=0x... \
cargo run --example 17_funding_to_external
POLYESTER_EXAMPLES_ENABLE_CHAIN_EXTERNAL_SUBMIT=1 \
POLYESTER_OWNER_PRIVATE_KEY=0x... \
POLYESTER_EXAMPLES_EXTERNAL_DESTINATION=0x... \
cargo run --example 17_funding_to_external
```

## Useful Settings

- `POLYESTER_EXAMPLES_SYMBOL`: default `BTC-USDT` (devnet has live BTC orderbook/candles; ETH-USDT may be quiet)
- `POLYESTER_EXAMPLES_TIMEFRAME`: default `1m`
- `POLYESTER_EXAMPLES_CANDLE_LIMIT`: default `100`
- `POLYESTER_EXAMPLES_MAX_QUOTE`: default `10`
- `POLYESTER_EXAMPLES_RSI_PERIOD`: default `14`
- `POLYESTER_EXAMPLES_RSI_OVERSOLD`: default `30`
- `POLYESTER_EXAMPLES_RSI_OVERBOUGHT`: default `70`
- `POLYESTER_EXAMPLES_ORDER_TIMEOUT_SEC`: default `15`
- `POLYESTER_EXAMPLES_STREAM_COUNT`: default `5`
- `POLYESTER_EXAMPLES_ORDERBOOK_DEPTH`: default `50`
- `POLYESTER_EXAMPLES_TRANSFER_AMOUNT`: default `0.01`
- `POLYESTER_EXAMPLES_WITHDRAW_AMOUNT`: default `0.01`
- `POLYESTER_EXAMPLES_CHAIN_AMOUNT`: default `1`
- `POLYESTER_EXAMPLES_EXTERNAL_DESTINATION`: required to encode example `17`
- `POLYESTER_EXAMPLES_EXTERNAL_CHAIN_ID`: default `6`
- `POLYESTER_OWNER_PRIVATE_KEY`: smart-account owner for UserOp submit (`16`/`17`)

## Notes For Bot Builders

- Pass `Price` / `Quantity` wrappers only. Do not use floats or bare integers for order inputs.
- Prefer decimal constructors for readable demos; prefer `from_ticks` / `from_scaled` when your bot
  already works in wire units.
- Size from ledger `available`, not gross `trading` (reservations reduce spendable).
- Use client order IDs for idempotency and cleanup.
- Check open orders before placing a new bot order.
- Decide how your production bot will track positions. The RSI example intentionally avoids a
  persistent state file and only uses balances plus open orders.
- Treat the RSI strategy as a teaching example. It is deliberately naive.

## Development

```bash
cargo test
cargo build --examples
make live-smoke
```
