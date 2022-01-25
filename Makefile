check: 
	cargo check
build: 
	cargo build
release:
	cargo build --release
fmt:
	cargo +nightly fmt
test:
	cargo test --all
lint:
	cargo clippy --all-targets -- -D warnings