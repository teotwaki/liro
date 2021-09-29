# Build stage
FROM rust:1.55 AS builder

RUN \
  apt-get update \
  && apt-get install -y \
    musl-tools \
    upx \
  && rustup target add x86_64-unknown-linux-musl \
  && cargo new /src
WORKDIR /src

# Cache dependencies. This will be invalidated with every version bump.
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release --target x86_64-unknown-linux-musl

# Build actual library & application
COPY src ./src
COPY templates ./templates
RUN \
  cargo build --release --target x86_64-unknown-linux-musl \
  && strip target/x86_64-unknown-linux-musl/release/liro \
  && upx --lzma target/x86_64-unknown-linux-musl/release/liro

FROM scratch
ENV RUST_LOG=info
ENV RUST_BACKTRACE=full

COPY assets /assets
COPY --from=builder /src/target/x86_64-unknown-linux-musl/release/liro /app

CMD ["./app"]
