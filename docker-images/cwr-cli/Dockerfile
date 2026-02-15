FROM ghcr.io/afbase/rust-zola-builder:pinned AS builder

WORKDIR /build
COPY . .
RUN cargo build --release --package cwr-cli

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/cwr-cli /usr/local/bin/cwr-cli
ENTRYPOINT ["cwr-cli"]
