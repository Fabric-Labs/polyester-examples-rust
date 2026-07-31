#!/usr/bin/env bash
# Run all Polyester Rust examples in order. Gated examples SKIP when their
# opt-in flag is unset, unless LIVE_SMOKE_STRICT=1 (then missing gates fail).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  source .env
  set +a
fi

STRICT="${LIVE_SMOKE_STRICT:-0}"
FAILED=0
SKIPPED=0
RAN=0

env_truthy() {
  local v
  v="$(echo "${1:-}" | tr '[:upper:]' '[:lower:]')"
  case "$v" in
    1|true|yes|on) return 0 ;;
    *) return 1 ;;
  esac
}

has_auth() {
  [[ -n "${POLYESTER_API_KEY_ID:-}" && -n "${POLYESTER_API_PRIVATE_KEY:-}" && -n "${POLYESTER_ACCOUNT_ID:-}" ]]
}

skip_or_fail() {
  local name="$1"
  local reason="$2"
  if env_truthy "$STRICT"; then
    echo "FAIL (strict): $name - $reason"
    FAILED=$((FAILED + 1))
  else
    echo "SKIP: $name - $reason"
    SKIPPED=$((SKIPPED + 1))
  fi
}

run_example() {
  local name="$1"
  echo ""
  echo "======== $name ========"
  if cargo run --example "$name"; then
    RAN=$((RAN + 1))
    echo "OK: $name"
  else
    echo "FAIL: $name"
    FAILED=$((FAILED + 1))
  fi
}

maybe_run() {
  local name="$1"
  local gate_ok="$2"
  local reason="$3"
  if [[ "$gate_ok" == "1" ]]; then
    run_example "$name"
  else
    skip_or_fail "$name" "$reason"
  fi
}

echo "Polyester Rust examples live-smoke (STRICT=${STRICT})"

# Public / read - always
run_example "01_public_market_data"
run_example "04_public_realtime_trades"
run_example "05_public_orderbook_stream"
run_example "06_market_overview_stream"

# Authenticated reads
if has_auth; then
  run_example "02_balances_and_orders_read"
  run_example "13_private_realtime"
else
  skip_or_fail "02_balances_and_orders_read" "missing API credentials"
  skip_or_fail "13_private_realtime" "missing API credentials"
fi

# RSI dry-run (auth optional when trading disabled)
run_example "10_rsi_signal_bot"

# Trading writes
TRADING=0
if env_truthy "${POLYESTER_EXAMPLES_ENABLE_TRADING:-}"; then
  TRADING=1
fi
maybe_run "03_place_and_cancel_limit_order" "$TRADING" "set POLYESTER_EXAMPLES_ENABLE_TRADING=1"
maybe_run "03b_scaled_int_limit_order" "$TRADING" "set POLYESTER_EXAMPLES_ENABLE_TRADING=1"
maybe_run "07_batch_create_and_cancel_all" "$TRADING" "set POLYESTER_EXAMPLES_ENABLE_TRADING=1"
maybe_run "08_batch_replace" "$TRADING" "set POLYESTER_EXAMPLES_ENABLE_TRADING=1"
maybe_run "09_batch_cancel" "$TRADING" "set POLYESTER_EXAMPLES_ENABLE_TRADING=1"
maybe_run "11_twap_trigger" "$TRADING" "set POLYESTER_EXAMPLES_ENABLE_TRADING=1"
maybe_run "12_ladder_trigger" "$TRADING" "set POLYESTER_EXAMPLES_ENABLE_TRADING=1"
maybe_run "18_trailing_stop_trigger" "$TRADING" "set POLYESTER_EXAMPLES_ENABLE_TRADING=1"

# Transfers - separate flag (never ENABLE_TRADING)
TRANSFERS=0
if env_truthy "${POLYESTER_EXAMPLES_ENABLE_TRANSFERS:-}" && [[ -n "${POLYESTER_EXAMPLES_TRANSFER_DEST_ACCOUNT_ID:-}" ]]; then
  TRANSFERS=1
fi
maybe_run "14_internal_transfer" "$TRANSFERS" "set POLYESTER_EXAMPLES_ENABLE_TRANSFERS=1 and POLYESTER_EXAMPLES_TRANSFER_DEST_ACCOUNT_ID"

# Withdraw prepare always when auth present; submit gated inside the example
if has_auth; then
  run_example "15_api_key_trading_withdraw"
else
  skip_or_fail "15_api_key_trading_withdraw" "missing API credentials"
fi

# Chain encode always with auth; submit gated inside each example
if has_auth; then
  run_example "16_funding_to_trading"
else
  skip_or_fail "16_funding_to_trading" "missing API credentials"
fi

if has_auth && [[ -n "${POLYESTER_EXAMPLES_EXTERNAL_DESTINATION:-}" ]]; then
  run_example "17_funding_to_external"
elif [[ -z "${POLYESTER_EXAMPLES_EXTERNAL_DESTINATION:-}" ]]; then
  skip_or_fail "17_funding_to_external" "set POLYESTER_EXAMPLES_EXTERNAL_DESTINATION"
else
  skip_or_fail "17_funding_to_external" "missing API credentials"
fi

echo ""
echo "live-smoke done: ran=$RAN skipped=$SKIPPED failed=$FAILED"
if [[ "$FAILED" -ne 0 ]]; then
  exit 1
fi
