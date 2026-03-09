const DEFAULT_UA =
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36';

function isLocalOrPrivateHost(hostname) {
  const host = (hostname || '').toLowerCase();

  if (!host) return true;
  if (host === 'localhost' || host === '0.0.0.0' || host === '::1') return true;
  if (host.endsWith('.localhost')) return true;

  if (/^\d+\.\d+\.\d+\.\d+$/.test(host)) {
    const parts = host.split('.').map((v) => Number(v));
    const [a, b] = parts;
    if (a === 10) return true;
    if (a === 127) return true;
    if (a === 169 && b === 254) return true;
    if (a === 172 && b >= 16 && b <= 31) return true;
    if (a === 192 && b === 168) return true;
  }

  return false;
}

function parseAllowlist(env) {
  const raw = env && env.UPSTREAM_ALLOWLIST ? String(env.UPSTREAM_ALLOWLIST) : '';
  return raw
    .split(',')
    .map((v) => v.trim().toLowerCase())
    .filter(Boolean);
}

function isAllowedByList(hostname, allowlist) {
  if (allowlist.length === 0) return true;
  const host = hostname.toLowerCase();
  return allowlist.some((entry) => host === entry || host.endsWith(`.${entry}`));
}

function corsHeaders() {
  return {
    'access-control-allow-origin': '*',
    'access-control-allow-methods': 'GET, OPTIONS',
    'access-control-allow-headers': 'content-type, accept, accept-language',
    'access-control-max-age': '86400',
  };
}

function json(status, payload) {
  return new Response(JSON.stringify(payload), {
    status,
    headers: {
      ...corsHeaders(),
      'content-type': 'application/json; charset=utf-8',
      'cache-control': 'no-store',
    },
  });
}

function cacheTtlFor(target, contentType) {
  const ct = String(contentType || '').toLowerCase();
  const path = target.pathname.toLowerCase();
  const isImage =
    ct.startsWith('image/') ||
    /\.(avif|webp|png|jpe?g|gif|bmp|svg)(\?|$)/i.test(path);
  const isHtml = ct.includes('text/html') || ct.includes('application/xhtml+xml');

  if (isImage) {
    return 'public, max-age=86400, s-maxage=86400, stale-while-revalidate=604800';
  }
  if (isHtml) {
    return 'public, max-age=30, s-maxage=120, stale-while-revalidate=600';
  }
  return 'public, max-age=60, s-maxage=300, stale-while-revalidate=1200';
}

function buildCacheKey(requestUrl, targetUrl) {
  const key = new URL(requestUrl);
  key.pathname = '/__edge_cache__/proxy';
  key.search = `u=${encodeURIComponent(targetUrl.toString())}`;
  return new Request(key.toString(), { method: 'GET' });
}

export default async function onRequest(context) {
  const { request, env } = context;

  if (request.method === 'OPTIONS') {
    return new Response(null, {
      status: 204,
      headers: corsHeaders(),
    });
  }

  if (request.method !== 'GET') {
    return json(405, { error: 'method not allowed' });
  }

  const reqUrl = new URL(request.url);
  const rawTarget = reqUrl.searchParams.get('url');
  if (!rawTarget) {
    return json(400, { error: 'missing url query param' });
  }

  let target;
  try {
    target = new URL(rawTarget);
  } catch (_) {
    return json(400, { error: 'invalid target url' });
  }

  if (target.protocol !== 'http:' && target.protocol !== 'https:') {
    return json(400, { error: 'only http/https is allowed' });
  }

  if (isLocalOrPrivateHost(target.hostname)) {
    return json(403, { error: 'private/local address is blocked' });
  }

  const allowlist = parseAllowlist(env);
  if (!isAllowedByList(target.hostname, allowlist)) {
    return json(403, { error: 'target host not allowed' });
  }

  const cache = typeof caches !== 'undefined' ? caches.default : null;
  const cacheKey = buildCacheKey(request.url, target);
  if (cache) {
    try {
      const cached = await cache.match(cacheKey);
      if (cached) {
        const hitHeaders = new Headers(cached.headers);
        hitHeaders.set('x-proxy-cache', 'HIT');
        return new Response(cached.body, {
          status: cached.status,
          statusText: cached.statusText,
          headers: hitHeaders,
        });
      }
    } catch (_) {
      // Continue without cache if cache API is unavailable.
    }
  }

  let upstream;
  try {
    upstream = await fetch(target.toString(), {
      method: 'GET',
      redirect: 'follow',
      headers: {
        accept: request.headers.get('accept') || '*/*',
        'accept-language': request.headers.get('accept-language') || 'en-US,en;q=0.9',
        'user-agent': DEFAULT_UA,
      },
    });
  } catch (err) {
    return json(502, {
      error: 'failed to fetch upstream',
      message: err && err.message ? err.message : String(err),
    });
  }

  const headers = new Headers(upstream.headers);
  const cors = corsHeaders();
  Object.keys(cors).forEach((key) => headers.set(key, cors[key]));
  headers.set('x-content-type-options', 'nosniff');
  headers.delete('set-cookie');
  headers.delete('set-cookie2');
  headers.delete('pragma');
  headers.set('x-proxy-cache', 'MISS');

  const cacheControl = cacheTtlFor(target, headers.get('content-type'));
  headers.set('cache-control', cacheControl);

  const response = new Response(upstream.body, {
    status: upstream.status,
    statusText: upstream.statusText,
    headers,
  });

  if (cache && upstream.status === 200) {
    const storePromise = cache.put(cacheKey, response.clone()).catch(() => {});
    if (typeof context.waitUntil === 'function') {
      context.waitUntil(storePromise);
    } else {
      await storePromise;
    }
  }

  return response;
}
