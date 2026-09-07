FROM rust:bookworm AS build
WORKDIR /src
COPY rust-toolchain.toml rust-toolchain.toml
RUN rustup show
COPY . .
RUN cargo build --release --workspace

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/auth-server /usr/local/bin/auth-server
COPY --from=build /src/target/release/world-server /usr/local/bin/world-server
COPY --from=build /src/target/release/gateway /usr/local/bin/gateway
COPY --from=build /src/target/release/map-server /usr/local/bin/map-server
COPY shards.yaml /etc/wow/shards.yaml
ENV SHARDS_FILE=/etc/wow/shards.yaml
WORKDIR /etc/wow
