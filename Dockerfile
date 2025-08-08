FROM rust:1.75 as builder
WORKDIR /usr/src/walrus

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY build.rs ./
COPY api ./api

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -r walrus && useradd -r -g walrus walrus

WORKDIR /app
COPY --from=builder /usr/src/walrus/target/release/walrus /app/walrus

RUN mkdir -p /data && chown -R walrus:walrus /data

USER walrus
EXPOSE 8080
ENTRYPOINT ["/app/walrus"]
