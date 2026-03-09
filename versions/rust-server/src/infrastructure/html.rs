use std::fmt::Write as _;

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use url::Url;

pub fn build_reader_html(
    title: &str,
    chapter_hash: &str,
    image_count: usize,
    next_link: Option<String>,
) -> String {
    let escaped_title = html_escape(title);
    let mut images_markup = String::new();

    for i in 1..=image_count {
        let _ = writeln!(
            images_markup,
            "<img class=\"page\" data-src=\"/assets/{}/{}.avif\" alt=\"Page {}\" decoding=\"async\" />",
            chapter_hash,
            format!("{:03}", i),
            i
        );
    }

    let next_button = next_link.map_or_else(
        || "<span class=\"btn disabled\">No Next Chapter</span>".to_string(),
        |href| format!("<a class=\"btn\" href=\"{href}\">Next Chapter</a>"),
    );

    format!(
        "<!doctype html>
<html lang=\"en\">
<head>
  <meta charset=\"utf-8\" />
  <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\" />
  <title>{escaped_title}</title>
  <style>
    :root {{ color-scheme: dark; }}
    body {{ margin: 0; background: #0f1115; color: #f8f8f8; font-family: Georgia, serif; }}
    .wrap {{ max-width: 920px; margin: 0 auto; padding: 18px 14px 36px; }}
    h1 {{ font-size: 1.25rem; margin: 0 0 14px; }}
    .meta {{ font-size: 0.9rem; color: #a3a9b6; margin-bottom: 14px; }}
    .page {{ display: block; width: 100%; margin: 0; background: #151922; min-height: 80px; }}
    .footer {{ display: flex; justify-content: center; padding-top: 16px; }}
    .btn {{ display: inline-block; background: #f59e0b; color: #1a1200; text-decoration: none; padding: 10px 14px; border-radius: 6px; font-weight: 700; }}
    .disabled {{ background: #4b5563; color: #e5e7eb; }}
  </style>
</head>
<body>
  <main class=\"wrap\">
    <h1>{escaped_title}</h1>
    <div class=\"meta\">Sequential image loading - AVIF cache</div>
    {images_markup}
    <div class=\"footer\">{next_button}</div>
  </main>
  <script>
    (() => {{
      const images = Array.from(document.querySelectorAll('img[data-src]'));
      let i = 0;
      const loadNext = () => {{
        if (i >= images.length) return;
        const img = images[i++];
        const src = img.getAttribute('data-src');
        if (!src) {{
          loadNext();
          return;
        }}
        img.onload = loadNext;
        img.onerror = loadNext;
        img.src = src;
      }};
      loadNext();
    }})();
  </script>
</body>
</html>"
    )
}

pub fn build_landing_html() -> String {
    "<!doctype html>
<html lang=\"en\">
<head>
  <meta charset=\"utf-8\" />
  <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\" />
  <title>Manga Mirror</title>
  <style>
    :root { color-scheme: dark; }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      min-height: 100vh;
      display: grid;
      place-items: center;
      background: radial-gradient(circle at 20% 20%, #1d2433 0%, #0d1119 45%, #07090f 100%);
      color: #f4f7ff;
      font-family: ui-sans-serif, Segoe UI, Tahoma, sans-serif;
      padding: 20px;
    }
    .card {
      width: 100%;
      max-width: 760px;
      background: rgba(10, 14, 22, 0.9);
      border: 1px solid #22304a;
      border-radius: 14px;
      padding: 24px;
      box-shadow: 0 20px 40px rgba(0, 0, 0, 0.35);
    }
    h1 { margin: 0 0 8px; font-size: 1.8rem; }
    p { margin: 0 0 16px; color: #b7c2d8; line-height: 1.5; }
    .input-row {
      display: grid;
      grid-template-columns: 1fr auto auto auto;
      gap: 10px;
    }
    input[type='url'] {
      width: 100%;
      padding: 12px 14px;
      border-radius: 10px;
      border: 1px solid #2b3b59;
      background: #111827;
      color: #f4f7ff;
      font-size: 0.98rem;
      outline: none;
    }
    input[type='url']:focus { border-color: #5fa8ff; }
    button {
      border: 0;
      border-radius: 10px;
      padding: 0 14px;
      cursor: pointer;
      font-weight: 700;
      min-height: 44px;
      white-space: nowrap;
    }
    .paste { background: #334155; color: #f8fafc; }
    .open { background: #f59e0b; color: #1f1400; }
    .open-raw { background: #22c55e; color: #05250f; }
    .hint { margin-top: 10px; font-size: 0.9rem; color: #94a3b8; }
    code {
      display: inline-block;
      margin-top: 8px;
      padding: 4px 8px;
      border-radius: 8px;
      background: #0f172a;
      border: 1px solid #23314d;
      color: #bfdbfe;
      font-size: 0.85rem;
    }
    @media (max-width: 680px) {
      .input-row { grid-template-columns: 1fr; }
      button { width: 100%; }
    }
  </style>
</head>
<body>
  <main class=\"card\">
    <h1>Manga Mirror</h1>
    <p>Paste URL chapter sumber. Pilih mode mirror AVIF atau mode raw streaming berurutan.</p>
    <form id=\"mirrorForm\" class=\"input-row\">
      <input id=\"targetUrl\" type=\"url\" placeholder=\"https://komiku.org/martial-peak-chapter-980/\" required />
      <button class=\"paste\" type=\"button\" id=\"pasteBtn\">Paste URL</button>
      <button class=\"open\" type=\"submit\" data-mode=\"mirror\">Open Mirror</button>
      <button class=\"open-raw\" type=\"submit\" data-mode=\"raw\">Open Raw</button>
    </form>
    <div class=\"hint\">
      Format endpoint:
      <code>/mirror/https://komiku.org/martial-peak-chapter-980/</code>
      <code>/raw/https://komiku.org/martial-peak-chapter-980/</code>
    </div>
  </main>
  <script>
    (() => {
      const form = document.getElementById('mirrorForm');
      const input = document.getElementById('targetUrl');
      const pasteBtn = document.getElementById('pasteBtn');

      pasteBtn.addEventListener('click', async () => {
        try {
          const text = await navigator.clipboard.readText();
          if (text) input.value = text.trim();
        } catch (_) {
          input.focus();
        }
      });

      form.addEventListener('submit', (e) => {
        e.preventDefault();
        const raw = input.value.trim();
        if (!raw) return;
        const mode = e.submitter && e.submitter.dataset && e.submitter.dataset.mode === 'raw'
          ? 'raw'
          : 'mirror';
        window.location.href = '/' + mode + '/' + encodeURIComponent(raw);
      });
    })();
  </script>
</body>
</html>"
        .to_string()
}

pub fn build_live_reader_html(chapter_hash: &str) -> String {
    format!(
        "<!doctype html>
<html lang=\"en\">
<head>
  <meta charset=\"utf-8\" />
  <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\" />
  <title>Loading chapter...</title>
  <style>
    :root {{ color-scheme: dark; }}
    body {{ margin: 0; background: #0f1115; color: #f8f8f8; font-family: Georgia, serif; }}
    .wrap {{ max-width: 920px; margin: 0 auto; padding: 14px 0 24px; }}
    h1 {{ font-size: 1.2rem; margin: 0 14px 10px; }}
    .meta {{ font-size: 0.85rem; color: #a3a9b6; margin: 0 14px 14px; }}
    .page {{ display: block; width: 100%; margin: 0; background: #151922; min-height: 80px; }}
    .footer {{ display: flex; justify-content: center; padding: 16px 14px 0; }}
    .btn {{ display: inline-block; background: #f59e0b; color: #1a1200; text-decoration: none; padding: 10px 14px; border-radius: 6px; font-weight: 700; }}
    .btn.disabled {{ background: #4b5563; color: #e5e7eb; pointer-events: none; }}
  </style>
</head>
<body>
  <main class=\"wrap\">
    <h1 id=\"chapterTitle\">Loading chapter...</h1>
    <div class=\"meta\" id=\"chapterMeta\">Preparing stream from upstream source...</div>
    <div id=\"pages\"></div>
    <div class=\"footer\"><a id=\"nextBtn\" class=\"btn disabled\" href=\"#\">No Next Chapter</a></div>
  </main>
  <script>
    (() => {{
      const chapterHash = '{chapter_hash}';
      const wsProto = location.protocol === 'https:' ? 'wss:' : 'ws:';
      const ws = new WebSocket(`${{wsProto}}//${{location.host}}/ws/${{chapterHash}}`);

      const titleEl = document.getElementById('chapterTitle');
      const metaEl = document.getElementById('chapterMeta');
      const pagesEl = document.getElementById('pages');
      const nextBtn = document.getElementById('nextBtn');
      const images = [];
      let initialized = false;
      let rawFirstThree = [];
      let rawRemaining = [];
      let totalImages = 0;

      let rawHeadIndex = 0;
      let rawTailIndex = 0;

      function setNextButton(href) {{
        if (!href) {{
          nextBtn.classList.add('disabled');
          nextBtn.removeAttribute('href');
          nextBtn.textContent = 'No Next Chapter';
          return;
        }}
        nextBtn.classList.remove('disabled');
        nextBtn.href = href;
        nextBtn.textContent = 'Next Chapter';
      }}

      function loadRawHeadSequential() {{
        if (!initialized) return;
        if (rawHeadIndex >= rawFirstThree.length) {{
          loadRawTailSequential();
          return;
        }}
        const img = images[rawHeadIndex];
        if (!img) return;
        const src = rawFirstThree[rawHeadIndex];
        rawHeadIndex++;
        img.onload = loadRawHeadSequential;
        img.onerror = loadRawHeadSequential;
        img.src = src;
      }}

      function loadRawTailSequential() {{
        if (!initialized) return;
        if (rawTailIndex >= rawRemaining.length) return;

        const absoluteIndex = rawTailIndex + 4;
        const img = images[absoluteIndex - 1];
        const src = rawRemaining[rawTailIndex];
        rawTailIndex++;

        if (!img || !src) {{
          loadRawTailSequential();
          return;
        }}
        if (img.getAttribute('src')) {{
          loadRawTailSequential();
          return;
        }}

        img.onload = loadRawTailSequential;
        img.onerror = loadRawTailSequential;
        img.src = src;
      }}

      function initChapter(payload) {{
        if (initialized) return;
        initialized = true;

        const chapterTitle = payload.title || 'Manga Chapter';
        totalImages = Number(payload.total_images || 0);
        rawFirstThree = (payload.raw_first_three || []).slice(0, 3);
        rawRemaining = Array.isArray(payload.raw_remaining) ? payload.raw_remaining : [];

        document.title = chapterTitle;
        titleEl.textContent = chapterTitle;
        metaEl.textContent = `Streaming mode: 3 raw images first, then sequential raw fallback (total ${{totalImages}} images)`;
        setNextButton(payload.next_mirror_path || null);

        for (let i = 1; i <= totalImages; i++) {{
          const img = document.createElement('img');
          img.className = 'page';
          img.id = `img-${{i}}`;
          img.alt = `Page ${{i}}`;
          img.decoding = 'async';
          pagesEl.appendChild(img);
          images.push(img);
        }}

        loadRawHeadSequential();
      }}

      function openDb() {{
        return new Promise((resolve, reject) => {{
          const req = indexedDB.open('mirror_prefetch_db', 1);
          req.onupgradeneeded = () => {{
            const db = req.result;
            if (!db.objectStoreNames.contains('chapters')) {{
              db.createObjectStore('chapters', {{ keyPath: 'chapter_hash' }});
            }}
          }};
          req.onsuccess = () => resolve(req.result);
          req.onerror = () => reject(req.error);
        }});
      }}

      async function idbPutChapter(payload) {{
        const db = await openDb();
        const tx = db.transaction('chapters', 'readwrite');
        const store = tx.objectStore('chapters');
        store.put(payload);
        await new Promise((resolve, reject) => {{
          tx.oncomplete = resolve;
          tx.onerror = () => reject(tx.error);
          tx.onabort = () => reject(tx.error);
        }});
      }}

      async function idbTrimMax20() {{
        const db = await openDb();
        const tx = db.transaction('chapters', 'readwrite');
        const store = tx.objectStore('chapters');
        const getAllReq = store.getAll();
        const all = await new Promise((resolve, reject) => {{
          getAllReq.onsuccess = () => resolve(getAllReq.result || []);
          getAllReq.onerror = () => reject(getAllReq.error);
        }});
        all.sort((a, b) => (a.cached_at || 0) - (b.cached_at || 0));
        while (all.length > 20) {{
          const old = all.shift();
          if (old && old.chapter_hash) store.delete(old.chapter_hash);
        }}
        await new Promise((resolve, reject) => {{
          tx.oncomplete = resolve;
          tx.onerror = () => reject(tx.error);
          tx.onabort = () => reject(tx.error);
        }});
      }}

      async function fetchImagesAsBlobEntries(urls) {{
        const out = [];
        for (const u of urls) {{
          try {{
            const r = await fetch(u);
            if (!r.ok) continue;
            const blob = await r.blob();
            out.push({{ url: u, blob }});
          }} catch (_) {{}}
        }}
        return out;
      }}

      ws.onmessage = async (evt) => {{
        let payload = null;
        try {{
          payload = JSON.parse(evt.data);
        }} catch (_) {{
          return;
        }}

        if (payload.type === 'chapter_init') {{
          initChapter(payload);
          return;
        }}

        if (payload.type === 'image_avif') {{
          const idx = Number(payload.index || 0);
          if (idx >= 4) {{
            const img = images[idx - 1];
            if (img && !img.getAttribute('src')) {{
              img.src = payload.url;
            }}
          }}
          return;
        }}

        if (payload.type === 'prefetched_chapter') {{
          const blobs = await fetchImagesAsBlobEntries(payload.image_urls || []);
          await idbPutChapter({{
            chapter_hash: payload.chapter_hash,
            source_url: payload.source_url,
            title: payload.title,
            images: blobs,
            cached_at: Date.now()
          }});
          await idbTrimMax20();
          return;
        }}

        if (payload.type === 'error') {{
          metaEl.textContent = `Failed to generate chapter: ${{payload.message || 'unknown error'}}`;
        }}
      }};
    }})();
  </script>
</body>
</html>"
    )
}

pub fn build_live_raw_reader_html(chapter_hash: &str) -> String {
    format!(
        "<!doctype html>
<html lang=\"en\">
<head>
  <meta charset=\"utf-8\" />
  <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\" />
  <title>Loading raw chapter...</title>
  <style>
    :root {{ color-scheme: dark; }}
    body {{ margin: 0; background: #0f1115; color: #f8f8f8; font-family: Georgia, serif; }}
    .wrap {{ max-width: 920px; margin: 0 auto; padding: 14px 0 24px; }}
    h1 {{ font-size: 1.2rem; margin: 0 14px 10px; }}
    .meta {{ font-size: 0.85rem; color: #a3a9b6; margin: 0 14px 14px; }}
    .page {{ display: block; width: 100%; margin: 0; background: #151922; min-height: 80px; }}
    .footer {{ display: flex; justify-content: center; padding: 16px 14px 0; }}
    .btn {{ display: inline-block; background: #22c55e; color: #062712; text-decoration: none; padding: 10px 14px; border-radius: 6px; font-weight: 700; }}
    .btn.disabled {{ background: #4b5563; color: #e5e7eb; pointer-events: none; }}
  </style>
</head>
<body>
  <main class=\"wrap\">
    <h1 id=\"chapterTitle\">Loading raw chapter...</h1>
    <div class=\"meta\" id=\"chapterMeta\">Preparing raw sequential stream from upstream source...</div>
    <div id=\"pages\"></div>
    <div class=\"footer\"><a id=\"nextBtn\" class=\"btn disabled\" href=\"#\">No Next Chapter</a></div>
  </main>
  <script>
    (() => {{
      const chapterHash = '{chapter_hash}';
      const wsProto = location.protocol === 'https:' ? 'wss:' : 'ws:';
      const wsUrl = `${{wsProto}}//${{location.host}}/ws/${{chapterHash}}`;

      const titleEl = document.getElementById('chapterTitle');
      const metaEl = document.getElementById('chapterMeta');
      const pagesEl = document.getElementById('pages');
      const nextBtn = document.getElementById('nextBtn');

      const images = [];
      const queue = [];
      const prefetchQueue = [];
      const seenPrefetchChapters = new Set();
      const activeBlobObjectUrls = [];

      let ws = null;
      let initialized = false;
      let totalImages = 0;
      let loadedCount = 0;
      let loadInProgress = false;
      let remainingQueued = false;
      let chapterFullyLoaded = false;
      let rawRemaining = [];
      let prefetchInProgress = false;
      let prefetchRequested = false;
      let pendingPrefetchRequest = false;
      let prefetchRequestPayload = '';
      let currentNextRawPath = null;
      let prefetchArrivalSeq = 0;

      function normalizeDecodedUrl(value) {{
        if (!value) return '';
        if (value.startsWith('https:/') && !value.startsWith('https://')) {{
          return value.replace('https:/', 'https://');
        }}
        if (value.startsWith('http:/') && !value.startsWith('http://')) {{
          return value.replace('http:/', 'http://');
        }}
        return value;
      }}

      function sourceUrlFromRawPath(pathname) {{
        if (!pathname || !pathname.startsWith('/raw/')) return null;
        const encoded = pathname.slice('/raw/'.length);
        if (!encoded) return null;
        try {{
          return normalizeDecodedUrl(decodeURIComponent(encoded));
        }} catch (_) {{
          return null;
        }}
      }}

      function normalizeSourceUrlKey(value) {{
        const raw = normalizeDecodedUrl((value || '').trim());
        if (!raw) return '';
        try {{
          const u = new URL(raw);
          u.hash = '';
          u.search = '';
          let path = u.pathname || '/';
          if (!path.endsWith('/')) path += '/';
          path = path.replace(/\\/{{2,}}/g, '/');
          return `${{u.protocol}}//${{u.host.toLowerCase()}}${{path}}`;
        }} catch (_) {{
          return raw
            .replace(/#.*$/, '')
            .replace(/\\?.*$/, '')
            .replace(/\\/?$/, '/');
        }}
      }}

      function chapterNumberFromSourceUrl(value) {{
        if (!value) return Number.MAX_SAFE_INTEGER;
        const match = /chapter-(\\d+)/i.exec(value);
        if (!match) return Number.MAX_SAFE_INTEGER;
        const num = Number(match[1]);
        return Number.isFinite(num) ? num : Number.MAX_SAFE_INTEGER;
      }}

      function clearBlobObjectUrls() {{
        while (activeBlobObjectUrls.length > 0) {{
          const url = activeBlobObjectUrls.pop();
          try {{
            URL.revokeObjectURL(url);
          }} catch (_) {{}}
        }}
      }}

      function setNextButton(href) {{
        if (!href) {{
          currentNextRawPath = null;
          nextBtn.classList.add('disabled');
          nextBtn.removeAttribute('href');
          nextBtn.textContent = 'No Next Chapter';
          return;
        }}
        currentNextRawPath = href;
        nextBtn.classList.remove('disabled');
        nextBtn.href = href;
        nextBtn.textContent = 'Next Chapter';
      }}

      function requestPrefetchIfNeeded() {{
        if (prefetchRequested) return;
        if (!currentNextRawPath) return;
        prefetchRequested = true;
        prefetchRequestPayload = JSON.stringify({{
          type: 'raw_prefetch_request',
          depth: 5,
          seed_next_raw_path: currentNextRawPath
        }});

        if (ws && ws.readyState === WebSocket.OPEN) {{
          ws.send(prefetchRequestPayload);
        }} else {{
          pendingPrefetchRequest = true;
        }}
      }}

      function resetChapterRender(title, total, nextRawPath) {{
        clearBlobObjectUrls();
        pagesEl.innerHTML = '';
        images.length = 0;
        queue.length = 0;
        rawRemaining = [];
        totalImages = Number(total || 0);
        loadedCount = 0;
        loadInProgress = false;
        remainingQueued = false;
        chapterFullyLoaded = false;
        prefetchRequested = false;
        pendingPrefetchRequest = false;
        prefetchRequestPayload = '';
        initialized = true;

        document.title = title || 'Raw Manga Chapter';
        titleEl.textContent = title || 'Raw Manga Chapter';
        metaEl.textContent = `Raw mode: sequential loading (total ${{totalImages}} images)`;
        setNextButton(nextRawPath || null);

        for (let i = 1; i <= totalImages; i++) {{
          const img = document.createElement('img');
          img.className = 'page';
          img.id = `img-${{i}}`;
          img.alt = `Page ${{i}}`;
          img.decoding = 'async';
          pagesEl.appendChild(img);
          images.push(img);
        }}
      }}

      function maybeQueueRemaining() {{
        if (remainingQueued) return;
        if (loadedCount < Math.min(3, totalImages)) return;
        remainingQueued = true;
        for (let i = 0; i < rawRemaining.length; i++) {{
          queue.push({{ index: i + 4, url: rawRemaining[i] }});
        }}
        tryLoadNext();
      }}

      function onCurrentChapterLoaded() {{
        if (chapterFullyLoaded) return;
        chapterFullyLoaded = true;
        metaEl.textContent = 'Raw chapter full-loaded. Requesting prefetch 5 next chapters...';
        requestPrefetchIfNeeded();
        drainPrefetchQueue();
      }}

      function tryLoadNext() {{
        if (!initialized || loadInProgress) return;
        const next = queue.shift();
        if (!next) {{
          if (loadedCount >= totalImages) onCurrentChapterLoaded();
          return;
        }}

        const img = images[next.index - 1];
        if (!img) {{
          tryLoadNext();
          return;
        }}

        loadInProgress = true;
        const done = () => {{
          loadInProgress = false;
          loadedCount++;
          maybeQueueRemaining();
          if (loadedCount >= totalImages) {{
            onCurrentChapterLoaded();
          }}
          tryLoadNext();
        }};

        img.onload = done;
        img.onerror = done;
        img.src = next.url;
      }}

      function renderFromNetworkPayload(payload) {{
        if (initialized) return;
        const title = payload.title || 'Raw Manga Chapter';
        const first = Array.isArray(payload.raw_first_three) ? payload.raw_first_three.slice(0, 3) : [];
        const remaining = Array.isArray(payload.raw_remaining) ? payload.raw_remaining : [];
        resetChapterRender(title, payload.total_images || 0, payload.next_raw_path || null);
        rawRemaining = remaining;
        for (let i = 0; i < first.length; i++) {{
          queue.push({{ index: i + 1, url: first[i] }});
        }}
        tryLoadNext();
      }}

      function renderFromCachedChapter(chapter) {{
        if (!chapter || !Array.isArray(chapter.images) || chapter.images.length === 0) return false;
        const blobItems = chapter.images.filter((item) => item && item.blob);
        if (blobItems.length === 0) return false;

        resetChapterRender(
          chapter.title || 'Raw Manga Chapter',
          blobItems.length,
          chapter.next_raw_path || null
        );

        const first = [];
        const remaining = [];
        for (let i = 0; i < blobItems.length; i++) {{
          const item = blobItems[i];
          const objectUrl = URL.createObjectURL(item.blob);
          activeBlobObjectUrls.push(objectUrl);
          if (i < 3) {{
            first.push(objectUrl);
          }} else {{
            remaining.push(objectUrl);
          }}
        }}
        rawRemaining = remaining;
        for (let i = 0; i < first.length; i++) {{
          queue.push({{ index: i + 1, url: first[i] }});
        }}
        tryLoadNext();
        return true;
      }}

      function openDb() {{
        return new Promise((resolve, reject) => {{
          const req = indexedDB.open('mirror_prefetch_db', 3);
          req.onupgradeneeded = () => {{
            const db = req.result;
            let store = null;
            if (!db.objectStoreNames.contains('chapters')) {{
              store = db.createObjectStore('chapters', {{ keyPath: 'chapter_hash' }});
            }} else {{
              store = req.transaction.objectStore('chapters');
            }}
            if (store && !store.indexNames.contains('source_url_idx')) {{
              store.createIndex('source_url_idx', 'source_url', {{ unique: false }});
            }}
            if (store && !store.indexNames.contains('source_key_idx')) {{
              store.createIndex('source_key_idx', 'source_key', {{ unique: false }});
            }}
          }};
          req.onsuccess = () => resolve(req.result);
          req.onerror = () => reject(req.error);
        }});
      }}

      async function idbPutChapter(payload) {{
        const db = await openDb();
        const tx = db.transaction('chapters', 'readwrite');
        const store = tx.objectStore('chapters');
        const row = {{
          ...payload,
          source_key: normalizeSourceUrlKey(payload && payload.source_url ? payload.source_url : '')
        }};
        store.put(row);
        await new Promise((resolve, reject) => {{
          tx.oncomplete = resolve;
          tx.onerror = () => reject(tx.error);
          tx.onabort = () => reject(tx.error);
        }});
      }}

      async function idbGetBySourceUrl(sourceUrl) {{
        if (!sourceUrl) return null;
        const sourceKey = normalizeSourceUrlKey(sourceUrl);
        const db = await openDb();
        const tx = db.transaction('chapters', 'readonly');
        const store = tx.objectStore('chapters');
        const allReq = store.getAll();
        const allRows = await new Promise((resolve, reject) => {{
          allReq.onsuccess = () => resolve(allReq.result || []);
          allReq.onerror = () => reject(allReq.error);
        }});
        const rows = (allRows || []).filter((row) => {{
          const rowUrl = row && row.source_url ? row.source_url : '';
          const rowKey = row && row.source_key ? row.source_key : normalizeSourceUrlKey(rowUrl);
          return rowKey === sourceKey || rowUrl === sourceUrl;
        }});

        rows.sort((a, b) => (b.cached_at || 0) - (a.cached_at || 0));
        const ready = rows.find((row) => Array.isArray(row.images) && row.images.length > 0);
        return ready || rows[0] || null;
      }}

      async function idbHasChapter(chapterHash) {{
        if (!chapterHash) return false;
        const db = await openDb();
        const tx = db.transaction('chapters', 'readonly');
        const store = tx.objectStore('chapters');
        const req = store.get(chapterHash);
        const row = await new Promise((resolve, reject) => {{
          req.onsuccess = () => resolve(req.result || null);
          req.onerror = () => reject(req.error);
        }});
        return !!row && Array.isArray(row.images) && row.images.length > 0;
      }}

      async function idbTrimMax20() {{
        const db = await openDb();
        const tx = db.transaction('chapters', 'readwrite');
        const store = tx.objectStore('chapters');
        const getAllReq = store.getAll();
        const all = await new Promise((resolve, reject) => {{
          getAllReq.onsuccess = () => resolve(getAllReq.result || []);
          getAllReq.onerror = () => reject(getAllReq.error);
        }});
        all.sort((a, b) => (a.cached_at || 0) - (b.cached_at || 0));
        while (all.length > 20) {{
          const old = all.shift();
          if (old && old.chapter_hash) store.delete(old.chapter_hash);
        }}
        await new Promise((resolve, reject) => {{
          tx.oncomplete = resolve;
          tx.onerror = () => reject(tx.error);
          tx.onabort = () => reject(tx.error);
        }});
      }}

      async function fetchBlobWithFallback(url) {{
        try {{
          const res = await fetch(url, {{ mode: 'cors', credentials: 'omit' }});
          if (res.ok) return await res.blob();
        }} catch (_) {{}}
        try {{
          const res = await fetch('/raw-image/' + encodeURIComponent(url));
          if (res.ok) return await res.blob();
        }} catch (_) {{}}
        return null;
      }}

      async function materializePrefetchedChapter(payload) {{
        if (!payload || !payload.chapter_hash) return;
        if (await idbHasChapter(payload.chapter_hash)) return;

        const urls = Array.isArray(payload.image_urls) ? payload.image_urls : [];
        const cachedImages = [];
        for (const u of urls) {{
          const blob = await fetchBlobWithFallback(u);
          if (!blob) continue;
          cachedImages.push({{ url: u, blob }});
        }}
        if (cachedImages.length === 0) return;

        await idbPutChapter({{
          chapter_hash: payload.chapter_hash,
          source_url: payload.source_url,
          title: payload.title,
          next_raw_path: payload.next_raw_path || null,
          images: cachedImages,
          cached_at: Date.now(),
        }});
        await idbTrimMax20();
      }}

      async function drainPrefetchQueue() {{
        if (prefetchInProgress) return;
        prefetchInProgress = true;
        while (prefetchQueue.length > 0) {{
          const payload = prefetchQueue.shift();
          try {{
            await materializePrefetchedChapter(payload);
          }} catch (_) {{}}
        }}
        prefetchInProgress = false;
      }}

      async function bootFromIndexedDbIfAny() {{
        const sourceUrl = sourceUrlFromRawPath(location.pathname);
        if (!sourceUrl) return;
        let cached = await idbGetBySourceUrl(sourceUrl);
        const shouldRetry =
          !cached &&
          typeof document.referrer === 'string' &&
          document.referrer.includes('/raw/');

        if (!cached && shouldRetry) {{
          for (let i = 0; i < 8; i++) {{
            await new Promise((resolve) => setTimeout(resolve, 120));
            cached = await idbGetBySourceUrl(sourceUrl);
            if (cached) break;
          }}
        }}

        if (!cached || initialized) return;
        if (renderFromCachedChapter(cached)) {{
          metaEl.textContent = 'Loaded chapter from local IndexedDB cache.';
        }}
      }}

      function startWs() {{
        ws = new WebSocket(wsUrl);
        ws.onopen = () => {{
          if (pendingPrefetchRequest && prefetchRequestPayload) {{
            ws.send(prefetchRequestPayload);
            pendingPrefetchRequest = false;
          }}
        }};
        ws.onmessage = async (evt) => {{
          let payload = null;
          try {{
            payload = JSON.parse(evt.data);
          }} catch (_) {{
            return;
          }}

          if (payload.type === 'raw_chapter_init') {{
            renderFromNetworkPayload(payload);
            return;
          }}

          if (payload.type === 'raw_prefetched_chapter') {{
            if (!payload.chapter_hash || seenPrefetchChapters.has(payload.chapter_hash)) {{
              return;
            }}
            seenPrefetchChapters.add(payload.chapter_hash);
            payload.__order = chapterNumberFromSourceUrl(payload.source_url || '');
            payload.__arrival = prefetchArrivalSeq++;
            prefetchQueue.push(payload);
            prefetchQueue.sort((a, b) => (a.__order - b.__order) || (a.__arrival - b.__arrival));
            drainPrefetchQueue();
            return;
          }}

          if (payload.type === 'error') {{
            metaEl.textContent = `Failed to load raw chapter: ${{payload.message || 'unknown error'}}`;
          }}
        }};
      }}

      (async () => {{
        await bootFromIndexedDbIfAny();
        startWs();
      }})();
    }})();
  </script>
</body>
</html>"
    )
}

pub fn mirror_path_for_url(url: &Url) -> String {
    let encoded = utf8_percent_encode(url.as_str(), NON_ALPHANUMERIC).to_string();
    format!("/mirror/{encoded}")
}

pub fn raw_path_for_url(url: &Url) -> String {
    let encoded = utf8_percent_encode(url.as_str(), NON_ALPHANUMERIC).to_string();
    format!("/raw/{encoded}")
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
