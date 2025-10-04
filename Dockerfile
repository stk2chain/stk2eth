# Multi-stage Dockerfile for STK2ETH Production
# Builds ussdclient and ethclient binaries

# ============================================================================
# Stage 1: Build Stage
# ============================================================================
FROM rust:1.83-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    musl-tools \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY ussdclient ./ussdclient
COPY ethclient ./ethclient

# Build release binaries with optimizations
RUN cargo build --release --workspace

# Strip binaries to reduce size
RUN strip target/release/ussdclient target/release/ethclient

# ============================================================================
# Stage 2: Runtime Stage for USSD Client
# ============================================================================
FROM debian:bookworm-slim AS ussdclient

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r ussd && useradd -r -g ussd ussd

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/ussdclient /usr/local/bin/ussdclient

# Set ownership
RUN chown -R ussd:ussd /app

# Switch to non-root user
USER ussd

# Expose USSD client port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the application
CMD ["ussdclient"]

# ============================================================================
# Stage 3: Runtime Stage for ETH Client
# ============================================================================
FROM debian:bookworm-slim AS ethclient

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r eth && useradd -r -g eth eth

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/ethclient /usr/local/bin/ethclient

# Set ownership
RUN chown -R eth:eth /app

# Switch to non-root user
USER eth

# Run the application
CMD ["ethclient"]

# ============================================================================
# Stage 4: All-in-One Stage (for development/testing)
# ============================================================================
FROM debian:bookworm-slim AS all-in-one

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    supervisor \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r stk2eth && useradd -r -g stk2eth stk2eth

WORKDIR /app

# Copy binaries from builder
COPY --from=builder /app/target/release/ussdclient /usr/local/bin/ussdclient
COPY --from=builder /app/target/release/ethclient /usr/local/bin/ethclient

# Create supervisor config
RUN mkdir -p /etc/supervisor/conf.d
COPY <<EOF /etc/supervisor/conf.d/stk2eth.conf
[supervisord]
nodaemon=true
user=root
logfile=/var/log/supervisor/supervisord.log
pidfile=/var/run/supervisord.pid

[program:ussdclient]
command=/usr/local/bin/ussdclient
user=stk2eth
autostart=true
autorestart=true
stdout_logfile=/dev/stdout
stdout_logfile_maxbytes=0
stderr_logfile=/dev/stderr
stderr_logfile_maxbytes=0

[program:ethclient]
command=/usr/local/bin/ethclient
user=stk2eth
autostart=true
autorestart=true
stdout_logfile=/dev/stdout
stdout_logfile_maxbytes=0
stderr_logfile=/dev/stderr
stderr_logfile_maxbytes=0
EOF

# Set ownership
RUN chown -R stk2eth:stk2eth /app

# Expose USSD client port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run supervisor
CMD ["/usr/bin/supervisord", "-c", "/etc/supervisor/supervisord.conf"]
