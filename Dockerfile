# Build stage
FROM ekidd/rust-musl-builder:1.55.0 AS builder

RUN USER=root cargo new /home/rust/src/liro
WORKDIR /home/rust/src/liro
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release

COPY src ./src
COPY templates ./templates
RUN cargo build --release

FROM debian:stable-slim
RUN apt-get update && apt-get install -y ca-certificates && apt-get clean
COPY assets /assets
COPY --from=builder /home/rust/src/liro/target/x86_64-unknown-linux-musl/release/liro /app
USER 1000
ENV RUST_LOG=info
ENV RUST_BACKTRACE=full
CMD ["./app"]
