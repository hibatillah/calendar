# syntax=docker/dockerfile:1
FROM rust:1.97.1-bookworm AS builder
WORKDIR /source
COPY Cargo.toml Cargo.lock ./
COPY apps/backend apps/backend
COPY apps/desktop/Cargo.toml apps/desktop/Cargo.toml
COPY crates crates
COPY assets/graphiql assets/graphiql
# Cargo resolves workspace manifests, but this image only compiles the backend.
RUN mkdir -p apps/desktop/src && touch apps/desktop/src/main.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/source/target \
    cargo build --locked --release -p calendar-backend && \
    cp target/release/calendar-backend /calendar-backend

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 10001 --create-home calendar \
    && mkdir -p /data && chown calendar:calendar /data
COPY --from=builder /calendar-backend /usr/local/bin/calendar-backend
USER calendar
WORKDIR /data
ENV CALENDAR_BIND_ADDRESS=0.0.0.0:47821 \
    CALENDAR_PRIVATE_SERVER=true \
    POSTHOG_ENABLED=false
EXPOSE 47821
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s \
    CMD curl --fail --silent http://127.0.0.1:47821/health > /dev/null || exit 1
ENTRYPOINT ["calendar-backend"]
