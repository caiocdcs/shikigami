FROM rust:1-bookworm AS builder
WORKDIR /app

# Precompile deps with a stub crate so source changes don't bust the dep layer.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
    && echo 'fn main() {}' > src/main.rs \
    && echo '' > src/lib.rs \
    && SQLX_OFFLINE=true cargo build --release \
    && rm -rf src target/release/shikigami*

COPY . .
RUN SQLX_OFFLINE=true cargo build --release \
    && strip target/release/shikigami

FROM debian:stable-slim AS runtime

RUN groupadd --system --gid 1000 shikigami \
    && useradd --system --uid 1000 --gid 1000 --home-dir /var/lib/shikigami --shell /usr/sbin/nologin shikigami \
    && install -d -o 1000 -g 1000 /var/lib/shikigami

WORKDIR /var/lib/shikigami

COPY --from=builder /app/target/release/shikigami /usr/local/bin/shikigami

ENV DATABASE_URL=sqlite:/var/lib/shikigami/shikigami.db?mode=rwc \
    LOG_LEVEL=info

USER 1000:1000

EXPOSE 3000
VOLUME ["/var/lib/shikigami"]

ENTRYPOINT ["/usr/local/bin/shikigami"]
