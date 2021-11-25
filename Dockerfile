# I'd prefer alpine, but there were weird interactions with libolm
FROM rust:slim-buster AS builder
RUN DEBIAN_FRONTEND=noninteractive apt update
RUN DEBIAN_FRONTEND=noninteractive apt install -y build-essential cmake pkg-config libssl-dev
WORKDIR /app

# Cache built dependencies unless Cargo.toml or Cargo.lock change
RUN cargo init
ADD Cargo.* ./
RUN cargo build --release
RUN rm -r src

# Now build the actual binary
ADD src/ src/
RUN touch src/main.rs
RUN cargo build --release
# Ensure it built correctly
RUN /app/target/release/rust-matrix-appservice-webhooks --help

FROM debian:buster-slim
RUN DEBIAN_FRONTEND=noninteractive apt update
RUN DEBIAN_FRONTEND=noninteractive apt install -y libssl-dev
COPY --from=builder /app/target/release/rust-matrix-appservice-webhooks /
RUN /rust-matrix-appservice-webhooks --help
ENTRYPOINT /rust-matrix-appservice-webhooks 