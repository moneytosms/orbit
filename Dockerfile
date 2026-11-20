# orbit-server image. Multi-arch via buildx (see scripts/release.sh).
FROM rust:1-bookworm AS web
RUN apt-get update && apt-get install -y --no-install-recommends nodejs npm \
 && npm i -g pnpm \
 && rustup target add wasm32-unknown-unknown \
 && cargo install --locked wasm-pack
# vite 8 / rolldown needs node >= 20.19; bookworm apt has 18
COPY --from=node:22-bookworm-slim /usr/local/bin/node /usr/local/bin/node
WORKDIR /src
COPY . .
RUN wasm-pack build crates/orbit-wasm --target web --out-dir ../../app/wasm --no-pack \
 && pnpm -C app install --frozen-lockfile && pnpm -C app build

FROM rust:1-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p orbit-server \
 && cp target/release/orbit-server /orbit-server

FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --home-dir /data orbit \
 && mkdir -p /data \
 && chown orbit:orbit /data
COPY --from=build /orbit-server /usr/local/bin/orbit-server
COPY --from=web /src/app/dist /web
ENV ORBIT_DATA=/data ORBIT_ADDR=0.0.0.0:8080 ORBIT_WEB=/web
USER orbit
WORKDIR /data
VOLUME ["/data"]
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/orbit-server"]
