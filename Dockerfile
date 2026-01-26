# RLM Server Dockerfile
#
# This Dockerfile builds the RLM (Recursive Language Model) server
# for production deployment with optimized size and security.

# Build stage - use official Rust image with necessary build tools
FROM rust:1.93.0-slim-bookworm AS builder

# Install system dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy Cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY crates/rlm-core/Cargo.toml crates/rlm-core/
COPY crates/rlm-repl-rhai/Cargo.toml crates/rlm-repl-rhai/
COPY crates/rlm-server/Cargo.toml crates/rlm-server/
COPY crates/rlm-ffi/Cargo.toml crates/rlm-ffi/
COPY crates/rlm-uar-adapter/Cargo.toml crates/rlm-uar-adapter/

# Create empty source files to cache dependencies
RUN mkdir -p crates/rlm-core/src crates/rlm-repl-rhai/src crates/rlm-server/src crates/rlm-ffi/src crates/rlm-uar-adapter/src && \
    echo "fn main() {}" > crates/rlm-core/src/lib.rs && \
    echo "fn main() {}" > crates/rlm-repl-rhai/src/lib.rs && \
    echo "fn main() {}" > crates/rlm-server/src/lib.rs && \
    echo "fn main() {}" > crates/rlm-server/src/main.rs && \
    echo "fn main() {}" > crates/rlm-ffi/src/lib.rs && \
    echo "fn main() {}" > crates/rlm-uar-adapter/src/lib.rs

# Build dependencies (this step will be cached)
RUN cargo build --release --bin rlm-server

# Remove the empty source files
RUN rm -rf crates/*/src

# Copy the actual source code
COPY crates/ crates/
COPY rustfmt.toml ./

# Build the application
RUN cargo build --release --bin rlm-server

# Runtime stage - use minimal Debian image
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user for security
RUN groupadd -r rlm && useradd -r -g rlm -s /bin/false rlm

# Set working directory
WORKDIR /app

# Copy the compiled binary from builder stage
COPY --from=builder /app/target/release/rlm-server /usr/local/bin/rlm-server

# Copy default configuration
COPY config.default.yaml /app/config.default.yaml

# Create directories for logs and data
RUN mkdir -p /app/logs /app/data && \
    chown -R rlm:rlm /app

# Switch to non-root user
USER rlm

# Expose the default port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Set environment variables
ENV RLM_SERVER_HOST=0.0.0.0
ENV RLM_SERVER_PORT=8080
ENV RLM_LOG_LEVEL=info
ENV RLM_LOG_FORMAT=json

# Default command
CMD ["rlm-server"]