# Build stage
FROM rust:1.78-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache dependencies
COPY Cargo.toml ./
COPY crates/shared/Cargo.toml ./crates/shared/
COPY crates/domain/Cargo.toml ./crates/domain/
COPY crates/customers/Cargo.toml ./crates/customers/
COPY crates/loans/Cargo.toml ./crates/loans/
COPY crates/accounts/Cargo.toml ./crates/accounts/
COPY crates/accounting/Cargo.toml ./crates/accounting/
COPY crates/payments/Cargo.toml ./crates/payments/
COPY crates/transactions/Cargo.toml ./crates/transactions/
COPY crates/charges/Cargo.toml ./crates/charges/
COPY crates/api-gateway/Cargo.toml ./crates/api-gateway/

# Create dummy source files for dependency caching
RUN for dir in shared domain customers loans accounts accounting payments transactions charges; do \
        mkdir -p crates/$dir/src && \
        echo "pub fn placeholder() {}" > crates/$dir/src/lib.rs; \
    done && \
    mkdir -p crates/api-gateway/src && \
    echo 'fn main() {}' > crates/api-gateway/src/main.rs

RUN cargo build --release --bin api-gateway 2>/dev/null || true

# Copy actual source
COPY . .

# Touch to force rebuild of changed sources
RUN find crates -name "*.rs" -exec touch {} \;

RUN cargo build --release --bin api-gateway

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/api-gateway ./api-gateway
COPY --from=builder /app/migrations ./migrations

EXPOSE 8080

ENV RUST_LOG=info

CMD ["./api-gateway"]
