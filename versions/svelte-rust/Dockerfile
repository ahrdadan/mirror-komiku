# syntax=docker/dockerfile:1.7

# As of 2026-03-13:
# - Node.js 24.14.0 is the latest LTS release (Active LTS).
# - Rust 1.94.0 is the latest stable release.
ARG NODE_VERSION=24.14.0
ARG RUST_VERSION=1.94.0

FROM rust:${RUST_VERSION}-bookworm AS backend-builder
WORKDIR /app/backend
COPY backend/ ./
RUN cargo build --release

FROM node:${NODE_VERSION}-bookworm-slim AS frontend-builder
WORKDIR /app/frontend
RUN corepack enable
COPY frontend/package.json frontend/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY frontend/ ./
RUN pnpm build

FROM debian:bookworm-slim AS app-runtime
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates nginx \
  && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=backend-builder /app/backend/target/release/mirror-komiku-proxy /usr/local/bin/mirror-komiku-proxy
COPY --from=frontend-builder /app/frontend/dist /usr/share/nginx/html
COPY docker/nginx.default.conf /etc/nginx/sites-available/default
COPY docker/start.sh /usr/local/bin/start.sh
RUN chmod +x /usr/local/bin/start.sh
EXPOSE 80
CMD ["/usr/local/bin/start.sh"]
