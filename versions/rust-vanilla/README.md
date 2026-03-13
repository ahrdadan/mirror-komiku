# Rust Server Version (`versions/rust-server`)

Server-side implementation of `mirror-komiku` using Rust + Actix Web.

[![Rust Docker Smoke Test](https://github.com/ahrdadan/mirror-komiku/actions/workflows/rust-docker-test.yml/badge.svg?branch=main)](https://github.com/ahrdadan/mirror-komiku/actions/workflows/rust-docker-test.yml)

## Repository

- GitHub: <https://github.com/ahrdadan/mirror-komiku>

## Overview

This version provides chapter mirroring and raw streaming with:

- HTTP + WebSocket endpoints
- dynamic generation pipeline
- cache + regeneration behavior
- background cleanup worker

## Features

- Fetch target chapter pages from `http/https` URL input
- Parse chapter title, image URLs, and next chapter URL
- Mirror mode:
  - download images
  - convert to AVIF
  - store generated assets in cache
- Raw mode:
  - sequential raw image behavior
  - prefetch metadata to client
- Cache strategy:
  - serve fresh cache when valid
  - serve stale + regenerate in background when expired
- Cleanup worker:
  - remove expired chapters
  - enforce maximum chapter count
  - enforce disk usage target
  - remove orphan temp/cache artifacts
- Basic SSRF hardening:
  - block localhost/private targets
  - optional domain allowlist

## Project Structure

- `src/main.rs` - composition root
- `src/config.rs` - environment/runtime config
- `src/domain/` - entities and parsing rules
- `src/application/` - orchestration/use-cases
- `src/infrastructure/` - IO adapters (network, storage, cleanup, html)
- `src/presentation/` - HTTP + WebSocket handlers

## Run

```bash
cargo run
```

Default run mode: `all`.

Other modes:

```bash
cargo run -- web
cargo run -- worker
```

Default bind:

- `http://0.0.0.0:7860`

## Endpoints

- Mirror by prefix:
  - `/mirror/https://komiku.org/martial-peak-chapter-980/`
- Raw by prefix:
  - `/raw/https://komiku.org/martial-peak-chapter-980/`
- Mirror direct fallback path:
  - `/https://komiku.org/martial-peak-chapter-980/`

## Environment Variables

- `BIND_ADDR` (default: `0.0.0.0`)
- `PORT` (default: `7860`)
- `CACHE_DIR` (default: `cache`)
- `CACHE_TTL_SECONDS` (default: `18000`)
- `MAX_CHAPTER_COUNT` (default: `20`)
- `DOWNLOAD_CONCURRENCY` (default: `4`)
- `ENCODE_CONCURRENCY` (default: `1`)
- `PREFETCH_DEPTH` (default: `3`)
- `CLEANUP_INTERVAL_SECONDS` (default: `300`)
- `RUN_MODE` (`web`, `worker`, `all`; default: `all`)
- `ALLOWED_DOMAINS` (default: `komiku.org,img.komiku.org`)

## Docker

Build:

```bash
docker build -t mirror-komiku:latest .
```

Run:

```bash
docker run --rm -p 7860:7860 -v ${PWD}/cache:/data/cache mirror-komiku:latest
```

## Load Test Script

```bash
bash scripts/load_test.sh "http://127.0.0.1:7860" "https://komiku.org/martial-peak-chapter-980/"
```

Override request/concurrency:

```bash
REQUESTS=40 CONCURRENCY=6 bash scripts/load_test.sh
```

## CI/CD

This version is validated by the root GitHub Actions workflow:

- Workflow: `.github/workflows/ci.yml`
- Job: `rust-server`
- Checks:
  - `cargo fmt --all -- --check`
  - `cargo check --all-targets --all-features`
  - `cargo test --all-targets --all-features -- --nocapture`

Dockerfile smoke testing:

- Workflow: `.github/workflows/rust-docker-test.yml`
- Checks:
  - `docker build` from `versions/rust-server/Dockerfile`
  - container run
  - running/exit-code
  - port `7860`
  - Docker `HEALTHCHECK`
