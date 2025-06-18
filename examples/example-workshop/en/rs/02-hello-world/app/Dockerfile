FROM rust:1.87 AS builder

WORKDIR /app

# Copy the Cargo.toml and src directory
COPY Cargo.toml .
COPY src ./src

# build the app
RUN cargo build

# Get the name of the application from Cargo.toml
RUN APP_NAME=$(awk '/^name = / {gsub(/"/, "", $3); print $3}' Cargo.toml) && \
    cp /app/target/debug/"$APP_NAME" /app/target/debug/lesson

# Final stage
FROM debian:bookworm-slim

# Install any runtime dependencies if needed
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /app/target/debug/lesson /app/lesson

# Set the command to run and redirect output
CMD ["/bin/sh", "-c", "/app/lesson > /app/stdout.log 2>&1"]
