# Dockerfile

ARG RUST_VERSION=1.78

# ---- Builder Stage ----
FROM rust:${RUST_VERSION}-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    sqlite3 \
    libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Diesel CLI with SQLite support
RUN cargo install diesel_cli --no-default-features --features sqlite

WORKDIR /app

# Copy only the manifests first to leverage Docker cache
COPY Cargo.toml Cargo.lock ./
COPY crates/rustash-core/Cargo.toml crates/rustash-core/
COPY crates/rustash-cli/Cargo.toml crates/rustash-cli/

# Create dummy files to build dependencies
RUN mkdir -p crates/rustash-core/src \
    && echo "fn main() {}" > crates/rustash-core/src/lib.rs \
    && mkdir -p crates/rustash-cli/src \
    && echo "fn main() {}" > crates/rustash-cli/src/main.rs

# Build dependencies
RUN cargo build --release --features sqlite

# Copy the actual source code
COPY . .

# Build the application
RUN touch crates/rustash-core/src/lib.rs \
    && touch crates/rustash-cli/src/main.rs \
    && cargo build --release --features sqlite

# ---- Final Stage ----
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder
COPY --from=builder /app/target/release/rustash /usr/local/bin/rustash

# Create a non-root user
RUN useradd -m rustash
USER rustash
WORKDIR /home/rustash

# Set the entrypoint
ENTRYPOINT ["rustash"]
CMD ["--help"]
