# Multi-stage build for battler-server
FROM rustlang/rust:nightly-bookworm AS builder

# Install Python and venv for battler-ai PyInstaller build step
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3 \
    python3-pip \
    python3-venv \
    libpython3.11 \
    python3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/battler

# Set up Python virtual environment with PyInstaller for battler-ai
COPY battler-ai-gemini-py ./battler-ai-gemini-py
RUN python3 -m venv /opt/venv \
    && /opt/venv/bin/pip install --no-cache-dir -r battler-ai-gemini-py/requirements.txt

ENV GEMINI_PYTHON_VENV=/opt/venv

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
    libpython3.11 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /usr/src/battler/target/release/battler-server /usr/local/bin/battler-server

# Copy pyinstaller bundle for battler-ai (matches compile-time embedded path)
COPY --from=builder /usr/src/battler/target/release/build /usr/src/battler/target/release/build

# Copy game data and descriptions
COPY battle-data ./battle-data

ENV RUST_LOG=info

EXPOSE 8080

ENTRYPOINT ["battler-server"]
CMD ["--address", "0.0.0.0", "--port", "8080", "--data-dir", "battle-data/data", "--descriptions-dir", "battle-data/descriptions"]
