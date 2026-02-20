# Stage 1: Build
FROM rust:1.93-slim AS builder

RUN apt-get update && apt-get install -y libssl-dev pkg-config && rm -rf /var/lib/apt/lists/*

WORKDIR /app

ENV SQLX_OFFLINE=true

COPY Cargo.toml Cargo.lock ./
COPY .sqlx .sqlx
COPY src src
COPY migrations migrations
COPY templates templates

RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/savethebeat /app/savethebeat
COPY --from=builder /app/migrations /app/migrations

EXPOSE 8080

CMD ["/app/savethebeat"]
