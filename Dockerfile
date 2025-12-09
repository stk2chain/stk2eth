# Dockerfile — production-ready multi-stage build for STK2ETH
# Builds ussdclient and ethclient release binaries and provides
# small runtime images for each service and an all-in-one dev image.

# ============================================================================
# Stage 0: Build deps (cache layer for cargo registry / dependencies)
# ============================================================================
FROM rustlang/rust:nightly-bookworm AS planner
WORKDIR /app

# Copy only manifests to leverage layer caching for dependencies
COPY Cargo.toml Cargo.lock ./
# Also copy crate-level Cargo files if present
COPY ussdclient/Cargo.toml ussdclient/Cargo.toml
COPY ethclient/Cargo.toml ethclient/Cargo.toml
COPY ussdgeth/Cargo.toml ussdgeth/Cargo.toml

# Create a dummy target to download dependencies
RUN mkdir -p src && echo "fn main(){}" > src/main.rs
RUN cargo fetch --locked || true

# ============================================================================
# Stage 1: Build Stage
# ============================================================================
FROM rustlang/rust:nightly-bookworm AS builder

# Install build dependencies needed for some crates (openssl, musl if needed)
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    ca-certificates \
    musl-tools \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests and source tree (workspace)
COPY Cargo.toml Cargo.lock ./
COPY ussdclient ./ussdclient
COPY ethclient ./ethclient
COPY ussdgeth ./ussdgeth
# Copy any top-level files needed for build
# Copy Cargo config if exists
RUN if [ -d ".cargo" ]; then cp -r .cargo /app/.cargo; fi
#COPY .cargo ./.cargo || true

# Build release binaries for the workspace
RUN cargo build --release --workspace --locked

# Strip binaries to reduce size
# (if strip not available in image, the command will be skipped)
RUN command -v strip >/dev/null 2>&1 && \
    strip target/release/ussdclient || true && \
    strip target/release/ethclient || true

# ============================================================================
# Stage 2: Runtime image for USSD client (small, non-root)
# ============================================================================
FROM debian:bookworm-slim AS ussd-runtime

# runtime deps
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# non-root user
RUN groupadd -r ussd && useradd -r -g ussd ussd

WORKDIR /app

# copy binary built earlier
COPY --from=builder /app/target/release/ussdclient /usr/local/bin/ussdclient

# ownership & permissions
RUN chown ussd:ussd /usr/local/bin/ussdclient && chmod 0755 /usr/local/bin/ussdclient

USER ussd

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -fsS --max-time 2 http://127.0.0.1:8080/health || exit 1

ENTRYPOINT ["/usr/local/bin/ussdclient"]

# ============================================================================
# Stage 3: Runtime image for ETH client (small, non-root)
# ============================================================================
FROM debian:bookworm-slim AS eth-runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -r eth && useradd -r -g eth eth

WORKDIR /app

COPY --from=builder /app/target/release/ethclient /usr/local/bin/ethclient

RUN chown eth:eth /usr/local/bin/ethclient && chmod 0755 /usr/local/bin/ethclient

USER eth

ENTRYPOINT ["/usr/local/bin/ethclient"]

# ============================================================================
# Stage 4: All-in-one image (for testing / small deployments)
# ============================================================================
FROM debian:bookworm-slim AS all-in-one

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    supervisor \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -r stk2eth && useradd -r -g stk2eth stk2eth

WORKDIR /app

COPY --from=builder /app/target/release/ussdclient /usr/local/bin/ussdclient
COPY --from=builder /app/target/release/ethclient /usr/local/bin/ethclient

RUN mkdir -p /etc/supervisor/conf.d
COPY <<'EOF' /etc/supervisor/conf.d/stk2eth.conf
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

RUN chown -R stk2eth:stk2eth /usr/local/bin/ussdclient /usr/local/bin/ethclient

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
  CMD curl -fsS --max-time 2 http://127.0.0.1:8080/health || exit 1

CMD ["/usr/bin/supervisord", "-c", "/etc/supervisor/supervisord.conf"]