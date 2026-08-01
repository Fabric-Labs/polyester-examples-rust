.PHONY: live-smoke test build

live-smoke:
	@bash scripts/live-smoke.sh

test:
	cargo test

build:
	cargo build --examples
