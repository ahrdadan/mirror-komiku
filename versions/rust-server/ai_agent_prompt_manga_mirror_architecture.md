# AI Agent Prompt - Mirror Komiku (Current Workspace Architecture)

## Objective

Build and maintain a Rust service that mirrors manga chapter pages from allowed upstream domains, converts images to AVIF, caches generated output, and serves fast reader pages with stale-while-regenerate behavior.

This document reflects the current workspace codebase in `d:\Project_\2026\mirror-komiku`.

---

## Runtime Entry

- Composition root: `src/main.rs`
- Web framework: Actix Web
- HTTP client: `reqwest`
- Async runtime: `tokio`
- Default run mode: `all` (web server + cleanup worker)

`reqwest` client must use this exact user agent:

```text
Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36
```

---

## Request Contract

Supported mirror routes:

- `/mirror/{target:.*}`
- `/{target:.*}` (fallback route)

Input is an encoded chapter URL, example:

```text
/mirror/https://komiku.org/martial-peak-chapter-980/
```

Validation requirements:

- Only `http` / `https`
- SSRF-safe URL checks
- Domain allowlist support (configurable)

---

## Current Source Parsing Rules

Parser module: `src/domain/parser.rs`

- Title extraction from document title/header
- Image extraction priority:
  - `#Baca_Komik img[src]`
  - fallback to global `img[src]`
- Next chapter URL extraction from chapter navigation links

Output model:

- `ParsedChapter { title, image_urls, next_url }`

---

## Caching Model

Filesystem roots (default `cache/`):

- `cache/pages/<chapter_hash>/index.html`
- `cache/pages/<chapter_hash>/meta.json`
- `cache/assets/<chapter_hash>/<nnn>.avif`

Metadata (`ChapterMeta`) includes:

- `source_url`
- `next_url`
- `generated_at`
- `expires_at`
- `title`
- `image_count`
- `total_bytes`

---

## Generation Behavior

### 1) Cache hit (fresh)

- Serve cached `index.html`
- Response header: `x-cache-status: HIT`

### 2) Cache hit (expired)

- Serve stale cached `index.html`
- Trigger background regeneration
- Response header: `x-cache-status: STALE`

### 3) Cache miss

- Start live streaming pipeline:
  - Fetch HTML
  - Parse chapter
  - Return live reader HTML immediately
  - First 3 images shown as upstream raw URLs
  - Remaining images converted to AVIF in background and pushed by WebSocket
- Response header: `x-cache-status: MISS_STREAMING`

If another request arrives during generation:

- Wait up to 45 seconds for generated page
- Serve with `x-cache-status: WAIT` when ready

---

## Live Pipeline + WebSocket

WebSocket endpoint:

- `/ws/{chapter_hash}`

Events (`WsEvent`):

- `image_avif`
- `chapter_done`
- `prefetched_chapter`
- `error`

Current flow:

1. Serve live HTML with raw first 3 images.
2. Convert tail images (`index >= 4`) to AVIF, emit `image_avif`.
3. Convert first 3 images to AVIF for final static cache.
4. Write `index.html` + `meta.json`.
5. Emit `chapter_done`.
6. Prefetch next chapters and emit `prefetched_chapter`.

---

## Prefetch Strategy

After chapter generation:

- Prefetch up to `PREFETCH_DEPTH` next chapters (default 3)
- For each prefetched chapter:
  - Generate full AVIF cache
  - Read resulting metadata
  - Emit chapter data over WebSocket so browser can store it (IndexedDB client-side logic exists in generated live HTML)

---

## Cleanup Worker (Current)

Module: `src/infrastructure/cleanup.rs`

Cleanup policies:

1. TTL expiry removal
2. Maximum chapter count enforcement (`MAX_CHAPTER_COUNT`)
3. Dynamic disk-limit enforcement (50% of available disk space)
4. Orphan directories + temp file cleanup

Logging policy (current request):

- Cleanup worker emits logs only for errors.
- Success/progress/info logs are intentionally removed.

---

## Module Map

`src/domain/`

- `models.rs`
- `parser.rs`

`src/application/`

- `state.rs`
- `ws_hub.rs`
- `chapter_service.rs`

`src/infrastructure/`

- `network.rs`
- `security.rs`
- `target.rs`
- `image.rs`
- `storage.rs`
- `cleanup.rs`
- `html.rs`

`src/presentation/`

- `http.rs`
- `ws.rs`

---

## Environment Variables

- `BIND_ADDR` (default `0.0.0.0`)
- `PORT` (default `7860`)
- `CACHE_DIR` (default `cache`)
- `CACHE_TTL_SECONDS` (default `18000`)
- `MAX_CHAPTER_COUNT` (default `20`)
- `DOWNLOAD_CONCURRENCY` (default `4`)
- `ENCODE_CONCURRENCY` (default `1`)
- `PREFETCH_DEPTH` (default `3`)
- `CLEANUP_INTERVAL_SECONDS` (default `300`)
- `RUN_MODE` (`web`, `worker`, `all`)
- `ALLOWED_DOMAINS` (default `komiku.org,img.komiku.org`)

---

## Non-Goals

- No full-site crawling.
- No public proxy behavior for arbitrary domains.
- No bypass of SSRF/domain validation rules.

