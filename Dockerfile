# Build stage
FROM rust:1.91 AS builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build the application in release mode
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install CA certificates for HTTPS requests
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/wind-tunnel-runner-status-dashboard .

# Expose the default port
EXPOSE 3000

# Set default environment variables
ENV BIND_ADDR=0.0.0.0:3000
ENV RUST_LOG=info

# Run the application
CMD ["./wind-tunnel-runner-status-dashboard"]
