FROM rust:1.89-bookworm AS development

RUN cargo install cargo-watch

WORKDIR /app

COPY Cargo.toml Cargo.lock ./

RUN mkdir src \
    && echo 'fn main() {}' > src/main.rs \
    && cargo build \
    && rm -rf src

COPY . .

CMD ["cargo", "watch", "-x", "run"]


FROM rust:1.89-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN --mount=type=cache,target=/app/target \
    --mount=type=cache,target=/var/cache/cargo \
    CARGO_HOME=/var/cache/cargo \
    cargo build --release \
    && cp /app/target/release/fshare-backend /usr/local/bin/fshare-backend


FROM debian:bookworm-slim AS production

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/bin/fshare-backend /usr/local/bin/fshare-backend

EXPOSE 9100

CMD ["/usr/local/bin/fshare-backend"]