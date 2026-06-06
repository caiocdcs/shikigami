default: lint build test

run:
    cargo run

test:
    cargo test

build:
    cargo build

release:
    cargo build --release

fmt:
    cargo fmt

fmt-check:
    cargo fmt --check

lint:
    cargo clippy -- -D warnings -D clippy::unwrap_used

lint-ci:
    cargo clippy -- -D warnings -D clippy::unwrap_used

audit:
    cargo audit

ci: fmt-check lint-ci test audit

sqlx-prepare:
    DATABASE_URL=sqlite:shikigami.db?mode=rwc cargo sqlx prepare

migrate:
    sqlx migrate run --database-url sqlite:shikigami.db?mode=rwc

clean:
    cargo clean

clean-all: clean
    rm -f shikigami.db shikigami.db-shm shikigami.db-wal
