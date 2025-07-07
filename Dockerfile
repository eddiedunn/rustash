# Dockerfile

# ---- Builder Stage ----
# Use the official Rust image as a build environment
FROM rust:1.78 as builder

# Install Diesel CLI for migrations
RUN cargo install diesel_cli --no-default-features --features "sqlite-bundled"

WORKDIR /app

# Copy manifests and lock file
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Create a dummy main.rs to cache dependencies
RUN mkdir -p crates/rustash-cli/src && echo "fn main() {}" > crates/rustash-cli/src/main.rs
RUN cargo build --release

# Copy the actual source code and build the application
COPY . .
RUN cargo build --release

# ---- Final Stage ----
# Use a slim, secure base image
FROM debian:buster-slim

# Install OpenSSL and ca-certificates which are required by Diesel
RUN apt-get update && apt-get install -y openssl ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/rustash /usr/local/bin/rustash

# Set the working directory
WORKDIR /app

# The default command to run when the container starts
CMD ["rustash", "list"]
