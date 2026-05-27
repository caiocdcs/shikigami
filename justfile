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

lint:
    cargo clippy -- -W clippy::pedantic
