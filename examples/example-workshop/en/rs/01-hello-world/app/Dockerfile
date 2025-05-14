FROM rust:1.82 AS builder

WORKDIR /app

# Copy the Cargo.toml and src directory
COPY ../../../Cargo.toml .
COPY ../../../src ./src

# Build the application
RUN cargo build

# Final stage
FROM debian:bookworm-slim

# Install any runtime dependencies if needed
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /app/target/debug/workshop /app/workshop

# Set the entrypoint
ENTRYPOINT ["/app/workshop"]

# This will be redirected to /stdout.log via the docker-compose.yaml
CMD [""]