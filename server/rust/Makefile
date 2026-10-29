all:
	cargo build --release
test:
	cargo test
check:
	cargo fmt --all -- --check
	cargo clippy --all-targets -- -D warnings
smoke: all
	CHAT_JWT_SECRET=test-only-not-production HTTP_PORT=18180 ./target/release/charlotte-server &
	sleep 1
	curl -fsS http://127.0.0.1:18180/healthz
