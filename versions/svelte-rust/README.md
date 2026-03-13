# Mirror Komiku v3 (Fullstack Proxy Reader)

Project root: `versions/svelte-rust`

This version is a fullstack architecture:

- Backend: Rust + Actix Web (safe lightweight proxy)
- Frontend: Svelte 5 + Vite + TypeScript
- Client cache: Dexie (IndexedDB) with LRU eviction
- Provider system: modular parser (`komiku` implemented)

## Folder Layout

```text
versions/svelte-rust
  backend/
    src/
      main.rs
      routes/
      providers/
      proxy/
      utils/
  frontend/
    src/
      components/
      providers/
      db/
      lib/
  shared/
    url-encode-utils/
```

## Routing Behavior

- Home: `/`
- Canonical chapter route: `/{provider}/{encoded}`
- Current provider route: `/komiku/{base64url(url)}`

Raw URL support:

- If user opens a path like:
  - `/https://komiku.org/martial-peak-chapter-981/`
- Frontend converts it into canonical:
  - `/komiku/<encoded>`

## Backend API

- Proxy (path style):
  - `GET /api/proxy/{provider}/{encoded}`
- Proxy (query style):
  - `GET /api/proxy?provider=komiku&u=<encoded>`
- Health:
  - `GET /health`

Backend proxy safeguards:

- allow only `http/https`
- provider domain matching (`komiku.org` and subdomain)
- block localhost/private targets
- DNS resolution check to block private-resolved addresses
- redirect chain limit
- response size limit
- upstream timeout

## Frontend Responsibilities

- Fetch chapter HTML through backend proxy
- Parse chapter using provider extractor
- Extract title, image URLs, next URL
- Render pages with horizontal slide interaction
- Sequential image unlock/loading
- Cache chapter metadata in IndexedDB
- Optional blob caching for images
- Prefetch next chapters in parallel

## Selector Notes (Komiku)

Implemented selectors follow `versions/static/SELECTORS.md`:

- Title:
  - `div header h1`
  - `h1`
- Images:
  - `#Baca_Komik img`
  - `div#Baca_Komik img`
  - `img`
- Next:
  - `a[rel='next']`
  - `a.next`
  - `.next a`
  - `.navig a`
  - `.pagination a`
  - `a`

## Run (Local)

### 1) Backend

```bash
cd versions/svelte-rust/backend
cargo run
```

Default bind: `127.0.0.1:8080`

Optional env:

- `BIND_ADDR` (default `127.0.0.1:8080`)
- `UPSTREAM_TIMEOUT_SECS` (default `12`)

### 2) Frontend

```bash
cd versions/svelte-rust/frontend
pnpm install
pnpm dev
```

Default frontend URL: `http://127.0.0.1:5173`

Vite proxies `/api/*` to backend `http://127.0.0.1:8080`.

## Docker (Standalone)

`versions/svelte-rust` now has standalone container files:

- `.gitignore`
- `.dockerignore`
- `Dockerfile` (multi-target)

LTS/stable base images pinned in Dockerfile (as of **March 13, 2026**):

- Node.js: `24.14.0` (latest LTS / Active LTS)
- Rust: `1.94.0` (latest stable)

Build backend image:

```bash
cd versions/svelte-rust
docker build --target backend-runtime -t mirror-komiku-v3-backend .
docker run --rm -p 8080:8080 mirror-komiku-v3-backend
```

Build frontend image:

```bash
cd versions/svelte-rust
docker build --target frontend-runtime -t mirror-komiku-v3-frontend .
docker run --rm -p 5173:80 mirror-komiku-v3-frontend
```
