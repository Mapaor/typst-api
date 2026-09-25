# Stage 1: Build the Rust application
FROM rust:1-slim-trixie AS builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends pkg-config libssl-dev curl && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# Create a dummy main.rs to cache dependencies
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && \
    echo "fn main() {println!(\"if you see this, the build failed\")}" > src/main.rs && \
    cargo build --release && \
    rm -rf src/

# Copy the actual source code and build the final binary
COPY src src
# Update timestamps to force rebuild of main.rs
RUN touch src/main.rs
RUN cargo build --release

# Stage 2: Final lightweight image
FROM debian:trixie-slim

# Install runtime dependencies (e.g., certificates and font packages)
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates fontconfig && \
    rm -rf /var/lib/apt/lists/*

# Copy curated default fonts into the system fonts directory
COPY assets/fonts /usr/share/fonts/typst

WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/app/target/release/typst-api /app/typst-api

# Create a default fonts directory
RUN mkdir -p /fonts
ENV TYPST_FONT_PATHS=/fonts

# Set default port
ENV PORT=8080
EXPOSE 8080

CMD ["/app/typst-api"]
