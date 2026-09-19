# Multi-stage build for battler-server
FROM rust:1.85-bookworm AS builder

WORKDIR /usr/src/battler

# Copy source workspace
COPY Cargo.toml Cargo.lock* ./
COPY battler ./battler
COPY battler-ai ./battler-ai
COPY battler-calc ./battler-calc
COPY battler-choice ./battler-choice
COPY battler-client ./battler-client
COPY battler-data ./battler-data
COPY battler-data-service ./battler-data-service
COPY battler-data-service-client ./battler-data-service-client
COPY battler-data-service-producer ./battler-data-service-producer
COPY battler-fuzz-test-generator ./battler-fuzz-test-generator
COPY battler-local-data ./battler-local-data
COPY battler-multiplayer-client ./battler-multiplayer-client
COPY battler-multiplayer-service ./battler-multiplayer-service
COPY battler-multiplayer-service-client ./battler-multiplayer-service-client
COPY battler-multiplayer-service-producer ./battler-multiplayer-service-producer
COPY battler-prng ./battler-prng
COPY battler-server ./battler-server
COPY battler-service ./battler-service
COPY battler-service-client ./battler-service-client
COPY battler-service-producer ./battler-service-producer
COPY battler-state ./battler-state
COPY battler-test-utils ./battler-test-utils
COPY battler-wamp ./battler-wamp
COPY battler-wamp-compat-test-bin ./battler-wamp-compat-test-bin
COPY battler-wamprat ./battler-wamprat
COPY battler-wamprat-schema ./battler-wamprat-schema

# Build release binary for battler-server
RUN cargo build --release -p battler-server

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /usr/src/battler/target/release/battler-server /usr/local/bin/battler-server

# Copy game data and descriptions
COPY battle-data ./battle-data

ENV RUST_LOG=info

EXPOSE 8080

ENTRYPOINT ["battler-server"]
CMD ["--address", "0.0.0.0", "--port", "8080", "--data-dir", "battle-data/data", "--descriptions-dir", "battle-data/descriptions"]
