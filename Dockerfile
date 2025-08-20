# Multi-stage Dockerfile for Solana Trading Bot
# Based on 2025 best practices for Rust applications

# ================================
# Stage 1: Build Dependencies Cache
# ================================
FROM rust:1.80-slim-bookworm AS chef
WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-chef for dependency caching
RUN cargo install cargo-chef

# ================================
# Stage 2: Dependency Planning
# ================================
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ================================
# Stage 3: Build Dependencies
# ================================
FROM chef AS builder

# Copy over dependency recipe
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies (this layer will be cached)
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source code
COPY . .

# Build application with 2025 optimizations
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true
ENV RUSTFLAGS="-C target-cpu=native -C opt-level=3 -C lto=fat -C codegen-units=1"

RUN cargo build --release --bin solana-trading-bot

# Strip debug symbols to reduce binary size
RUN strip target/release/solana-trading-bot

# ================================
# Stage 4: Runtime Environment
# ================================
FROM debian:bookworm-slim AS runtime

# Create non-root user for security
RUN groupadd --gid 1000 botuser \
    && useradd --uid 1000 --gid botuser --shell /bin/bash --create-home botuser

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libsqlite3-0 \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create application directory
WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /app/target/release/solana-trading-bot /usr/local/bin/solana-trading-bot

# Create necessary directories
RUN mkdir -p /app/data /app/logs && chown -R botuser:botuser /app

# Copy migrations if they exist
COPY --chown=botuser:botuser migrations ./migrations/ || true

# Switch to non-root user
USER botuser

# Set environment variables
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1
ENV DATABASE_URL=sqlite:///app/data/bot.db
ENV PORT=8080

# Expose port for health checks
EXPOSE 8080

# Enhanced health check with HTTP endpoint
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Labels for metadata and container management
LABEL maintainer="Solana Trading Bot Team"
LABEL version="1.0.0"
LABEL description="Production-ready Solana Trading Bot with Telegram interface"
LABEL org.opencontainers.image.source="https://github.com/your-org/solana-trading-bot"
LABEL org.opencontainers.image.title="Solana Trading Bot"
LABEL org.opencontainers.image.description="Advanced Solana trading bot with Jupiter integration"
LABEL org.opencontainers.image.vendor="Solana Trading Bot Team"
LABEL org.opencontainers.image.licenses="MIT"

# Graceful shutdown support
STOPSIGNAL SIGTERM

# Start the application
CMD ["solana-trading-bot"]