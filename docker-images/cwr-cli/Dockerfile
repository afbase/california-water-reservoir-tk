FROM ghcr.io/afbase/rust-zola-builder:pinned AS builder

WORKDIR /build
COPY . .
RUN cargo build --release --locked --package cwr-cli

# Must be the same distro release as the builder image (ubuntu:noble): the
# binary links against the builder's glibc and will not start on an older one.
FROM ubuntu:noble
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/cwr-cli /usr/local/bin/cwr-cli
ENTRYPOINT ["cwr-cli"]
