# Use the official Rust image as the base image
FROM rust:latest AS builder

# Set the working directory inside the container
WORKDIR /usr/src/app

# Copy the Cargo.toml and Cargo.lock files
COPY Cargo.toml Cargo.lock ./

# Copy the source code
COPY . .

# Build the application
RUN cargo build --release

# Use a minimal base image for the final container
FROM debian:bookworm-slim

# Set the working directory inside the container
WORKDIR /usr/src/app

# Install necessary dependencies
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy the built binary from the builder stage
COPY --from=builder /usr/src/app/target/release/WordWeaverBackend .
RUN mkdir assets
COPY --from=builder /usr/src/app/assets ./assets

# Expose the port that the application will run on
EXPOSE 3000

# Set the entrypoint to run the application
CMD ["./WordWeaverBackend"]