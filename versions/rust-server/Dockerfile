FROM rust:1.88-bookworm AS builder

WORKDIR /app
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && curl -fsSL https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64 \
      -o /usr/local/bin/cloudflared \
    && chmod +x /usr/local/bin/cloudflared \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/mirror-komiku /usr/local/bin/mirror-komiku

ENV BIND_ADDR=0.0.0.0
ENV PORT=7860
ENV RUN_MODE=all
ENV CACHE_DIR=/data/cache
ENV CACHE_TTL_SECONDS=18000
ENV MAX_CHAPTER_COUNT=20
ENV DOWNLOAD_CONCURRENCY=4
ENV ENCODE_CONCURRENCY=1
ENV PREFETCH_DEPTH=3
ENV CLEANUP_INTERVAL_SECONDS=300
ENV ALLOWED_DOMAINS=komiku.org,img.komiku.org
ENV RUST_LOG=info
ENV CF_TOKEN=eyJhIjoiYzIyMGE0YzAzZjZiOWFhNGUzYTYwZWU0MTIxZWMwNGMiLCJ0IjoiMWM4NGNlZjUtMWUyMi00ZTMyLThkMzgtODU5NDc4MGUwNjI0IiwicyI6IlpqUmhZVFJqTVRndE9EUXhOaTAwWmpneUxUZzBNV1V0T0dWaFlqY3hNRFF4WW1FNSJ9

EXPOSE 7860
RUN mkdir -p /data/cache
VOLUME ["/data/cache"]

CMD ["sh", "-lc", "if [ -z \"$CF_TOKEN\" ]; then echo \"CF_TOKEN is empty. Set CF_TOKEN for cloudflared tunnel.\"; exit 1; fi; echo \"starting mirror-komiku mode=$RUN_MODE bind=$BIND_ADDR port=$PORT cache=$CACHE_DIR\"; /usr/local/bin/mirror-komiku & APP_PID=$!; echo \"starting cloudflared tunnel to http://127.0.0.1:$PORT\"; cloudflared tunnel --no-autoupdate run --token \"$CF_TOKEN\"; EXIT_CODE=$?; echo \"cloudflared exited code=$EXIT_CODE\"; kill $APP_PID 2>/dev/null || true; wait $APP_PID 2>/dev/null || true; exit $EXIT_CODE"]
