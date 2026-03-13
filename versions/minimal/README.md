# Static Version (`versions/static`)

This version is fully independent from `versions/rust-server` and is designed for static hosting with EdgeOne Functions.

## Repository

- GitHub: <https://github.com/ahrdadan/mirror-komiku>
- Live URL: <https://mirror-komiku.edgeone.dev/>

## Architecture

- Static frontend:
  - `index.html`
  - `styles.css`
  - `app.js`
- Edge function proxy:
  - `edge-functions/api/proxy.js`

The frontend handles chapter parsing, sequential rendering, prefetch orchestration, and IndexedDB storage.

## Routes

- Homepage: `/`
- Reader: `/<encoded_target_url>`
- Proxy function: `/api/proxy?url=<upstream_url>`

Reader URL example:

- `/https%3A%2F%2Fkomiku.org%2Fmartial-peak-chapter-980%2F`

## Runtime Flow

1. User opens homepage and submits a Komiku chapter URL.
2. App redirects to `/<encoded_target_url>`.
3. Browser fetches chapter HTML through `/api/proxy`.
4. App extracts title, image URLs, and next chapter URL.
5. Current chapter images render sequentially.
6. After full load, app prefetches up to 5 next chapters in order.
7. Prefetched images are stored as blobs in IndexedDB.
8. On navigation, app uses local-first loading from IndexedDB.

## Caching

### Local Browser Cache (IndexedDB)

- DB: `mirror_raw_static_db`
- Store: `chapters`
- Max cached chapters: `10`
- Eviction: oldest entry first (`cached_at` ascending)

### Edge Proxy Cache

The proxy uses cache primitives to reduce upstream and function pressure:

- Cache API (`caches.default.match` / `caches.default.put`)
- Adaptive `Cache-Control` by content type
- `x-proxy-cache: HIT|MISS` response header

## Request Optimization

Implemented optimizations to minimize function requests:

- In-memory chapter manifest cache per session
- In-flight request de-duplication (text and blob)
- Retry with exponential backoff
- Failed-attempt cooldown to avoid repeated hot-fail requests
- Direct-image-first rendering with proxy fallback on failure

## EdgeOne Deployment Notes

1. Deploy folder `versions/static`.
2. Ensure `edge-functions` is included in deployment.
3. Configure SPA rewrite:
   - non-file routes -> `/index.html`
   - exclude `/api/*` from SPA rewrite
4. Optional security env:
   - `UPSTREAM_ALLOWLIST=komiku.org,img.komiku.org`

## Security Notes

The proxy only allows `http/https` targets and blocks local/private addresses.

## Local Testing

- `npx serve -s` serves static files only.
- It does not execute Edge Functions.
- End-to-end CORS validation requires EdgeOne Functions runtime.

## EdgeOne References

- Edge Functions lifecycle / `waitUntil`: <https://pages.edgeone.ai/document/161779710200866816>
- `edgeone.json` route/header/cache rules: <https://pages.edgeone.ai/document/162261069035167744>
- Cache API (`cache.match`, `cache.put`): <https://intl.cloud.tencent.com/document/product/1145/47615>

## CI/CD

This version is validated by the root GitHub Actions workflow:

- Workflow: `.github/workflows/ci.yml`
- Job: `static`
- Checks:
  - required static files exist
  - JavaScript syntax check for:
    - `versions/static/app.js`
    - `versions/static/edge-functions/api/proxy.js`
