# orbit-server image. Multi-arch via buildx (see scripts/release.sh).
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
ENV ORBIT_DATA=/data ORBIT_ADDR=0.0.0.0:8080
USER orbit
WORKDIR /data
VOLUME ["/data"]
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/orbit-server"]
