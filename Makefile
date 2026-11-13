.PHONY: all test check fmt release

all:
	cargo build

fmt:
	cargo fmt --all

check:
	cargo fmt --all -- --check
	cargo clippy --all-targets -- -D warnings
	cargo test --all-targets

release:
	cargo build --release

test:
	cargo test --all-targets
