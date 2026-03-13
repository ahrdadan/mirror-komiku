(() => {
  const DB_NAME = 'mirror_raw_static_db';
  const DB_VERSION = 1;
  const STORE_NAME = 'chapters';
  const MAX_CACHED_CHAPTERS = 10;
  const PREFETCH_DEPTH = 5;
  const MANIFEST_CACHE_LIMIT = 24;
  const PROXY_ENDPOINT = '/api/proxy?url=';
  const FETCH_RETRY_MAX = 3;
  const FETCH_BACKOFF_BASE_MS = 220;
  const FAILED_ATTEMPT_BASE_COOLDOWN_MS = 5000;
  const FAILED_ATTEMPT_MAX_COOLDOWN_MS = 300000;

  const FETCH_BUILDERS = [
    (url) => `${PROXY_ENDPOINT}${encodeURIComponent(url)}`,
  ];

  const inputPanel = document.getElementById('inputPanel');
  const readerPanel = document.getElementById('readerPanel');
  const openForm = document.getElementById('openForm');
  const targetInput = document.getElementById('targetUrl');
  const chapterTitleEl = document.getElementById('chapterTitle');
  const chapterMetaEl = document.getElementById('chapterMeta');
  const pagesEl = document.getElementById('pages');
  const nextBtn = document.getElementById('nextBtn');

  const state = {
    activeObjectUrls: [],
    loadToken: 0,
    prefetchRunning: false,
    pendingPrefetchStart: null,
    chapterManifestCache: new Map(),
    inflightTextRequests: new Map(),
    inflightBlobRequests: new Map(),
    failedAttempts: new Map(),
  };

  function setMeta(message) {
    chapterMetaEl.textContent = message;
  }

  function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  async function withInflight(map, key, task) {
    if (map.has(key)) {
      return map.get(key);
    }
    const promise = Promise.resolve()
      .then(task)
      .finally(() => {
        map.delete(key);
      });
    map.set(key, promise);
    return promise;
  }

  function normalizeDecodedUrl(value) {
    if (!value) return '';
    const trimmed = value.trim();
    if (trimmed.startsWith('https:/') && !trimmed.startsWith('https://')) {
      return trimmed.replace('https:/', 'https://');
    }
    if (trimmed.startsWith('http:/') && !trimmed.startsWith('http://')) {
      return trimmed.replace('http:/', 'http://');
    }
    return trimmed;
  }

  function normalizeSourceUrlKey(value) {
    const normalized = normalizeDecodedUrl(value);
    if (!normalized) return '';
    try {
      const url = new URL(normalized);
      url.hash = '';
      url.search = '';
      let path = url.pathname || '/';
      if (!path.endsWith('/')) path += '/';
      path = path.replace(/\/{2,}/g, '/');
      return `${url.protocol}//${url.host.toLowerCase()}${path}`;
    } catch (_) {
      return normalized.toLowerCase();
    }
  }

  function targetPathForUrl(url) {
    return `/${encodeURIComponent(url)}`;
  }

  function proxyUrlFor(url) {
    return `${PROXY_ENDPOINT}${encodeURIComponent(url)}`;
  }

  function cloneChapterManifest(chapter) {
    return {
      sourceUrl: chapter.sourceUrl,
      sourceKey: chapter.sourceKey,
      chapterHash: chapter.chapterHash,
      title: chapter.title,
      imageUrls: Array.isArray(chapter.imageUrls) ? [...chapter.imageUrls] : [],
      nextUrl: chapter.nextUrl || null,
    };
  }

  function cacheChapterManifest(chapter) {
    if (!chapter || !chapter.sourceKey) return;
    state.chapterManifestCache.delete(chapter.sourceKey);
    state.chapterManifestCache.set(chapter.sourceKey, cloneChapterManifest(chapter));
    while (state.chapterManifestCache.size > MANIFEST_CACHE_LIMIT) {
      const oldestKey = state.chapterManifestCache.keys().next().value;
      if (!oldestKey) break;
      state.chapterManifestCache.delete(oldestKey);
    }
  }

  function isAttemptCoolingDown(attempt) {
    const entry = state.failedAttempts.get(attempt);
    if (!entry) return false;
    if ((entry.until || 0) <= Date.now()) {
      state.failedAttempts.delete(attempt);
      return false;
    }
    return true;
  }

  function markAttemptFailure(attempt) {
    const current = state.failedAttempts.get(attempt);
    const nextCount = (current ? current.count : 0) + 1;
    const cooldown = Math.min(
      FAILED_ATTEMPT_MAX_COOLDOWN_MS,
      FAILED_ATTEMPT_BASE_COOLDOWN_MS * 2 ** Math.min(nextCount - 1, 6)
    );
    state.failedAttempts.set(attempt, {
      count: nextCount,
      until: Date.now() + cooldown,
    });
  }

  function clearAttemptFailure(attempt) {
    state.failedAttempts.delete(attempt);
  }

  async function fetchTextAttempt(attempt) {
    let lastError = null;
    let lastStatus = 0;
    for (let i = 0; i < FETCH_RETRY_MAX; i += 1) {
      try {
        const response = await fetch(attempt, { credentials: 'omit' });
        if (response.ok) {
          const text = await response.text();
          if (text && text.trim().length > 50) {
            clearAttemptFailure(attempt);
            return text;
          }
          lastError = new Error(`empty/invalid text payload from ${attempt}`);
          lastStatus = response.status;
        } else {
          lastStatus = response.status;
          if (response.status === 404 && attempt.startsWith(PROXY_ENDPOINT)) {
            throw new Error('proxy endpoint /api/proxy not found');
          }
          lastError = new Error(`status ${response.status} for ${attempt}`);
          // 4xx (except 429) generally should not be retried aggressively.
          if (response.status < 500 && response.status !== 429) {
            break;
          }
        }
      } catch (err) {
        lastError = err;
      }
      if (i < FETCH_RETRY_MAX - 1) {
        const jitter = Math.floor(Math.random() * 90);
        await sleep(FETCH_BACKOFF_BASE_MS * 2 ** i + jitter);
      }
    }
    if (lastStatus !== 404) {
      markAttemptFailure(attempt);
    }
    throw lastError || new Error(`failed text request for ${attempt}`);
  }

  async function fetchBlobAttempt(attempt) {
    let lastError = null;
    let lastStatus = 0;
    for (let i = 0; i < FETCH_RETRY_MAX; i += 1) {
      try {
        const response = await fetch(attempt, { credentials: 'omit' });
        if (response.ok) {
          const blob = await response.blob();
          if (blob && blob.size > 0) {
            clearAttemptFailure(attempt);
            return blob;
          }
          lastError = new Error(`empty blob payload from ${attempt}`);
          lastStatus = response.status;
        } else {
          lastStatus = response.status;
          if (response.status === 404 && attempt.startsWith(PROXY_ENDPOINT)) {
            throw new Error('proxy endpoint /api/proxy not found');
          }
          lastError = new Error(`status ${response.status} for ${attempt}`);
          if (response.status < 500 && response.status !== 429) {
            break;
          }
        }
      } catch (err) {
        lastError = err;
      }
      if (i < FETCH_RETRY_MAX - 1) {
        const jitter = Math.floor(Math.random() * 90);
        await sleep(FETCH_BACKOFF_BASE_MS * 2 ** i + jitter);
      }
    }
    if (lastStatus !== 404) {
      markAttemptFailure(attempt);
    }
    throw lastError || new Error(`failed blob request for ${attempt}`);
  }

  function setNextButton(nextUrl) {
    if (!nextUrl) {
      nextBtn.classList.add('disabled');
      nextBtn.removeAttribute('href');
      nextBtn.textContent = 'No Next Chapter';
      return;
    }
    nextBtn.classList.remove('disabled');
    nextBtn.href = targetPathForUrl(nextUrl);
    nextBtn.textContent = 'Next Chapter';
  }

  function showReader(targetUrl) {
    inputPanel.classList.add('hidden');
    readerPanel.classList.remove('hidden');
    document.body.classList.add('reading-mode');
    if (targetUrl) {
      targetInput.value = targetUrl;
    }
  }

  function showInput() {
    inputPanel.classList.remove('hidden');
    readerPanel.classList.add('hidden');
    document.body.classList.remove('reading-mode');
    setMeta('Waiting for chapter URL...');
  }

  function revokeActiveObjectUrls() {
    while (state.activeObjectUrls.length > 0) {
      const objectUrl = state.activeObjectUrls.pop();
      try {
        URL.revokeObjectURL(objectUrl);
      } catch (_) {
        // no-op
      }
    }
  }

  function resetPages() {
    revokeActiveObjectUrls();
    pagesEl.innerHTML = '';
  }

  function getInitialTargetFromLocation() {
    const queryTarget = new URLSearchParams(window.location.search).get('target');
    if (queryTarget) {
      const normalizedQuery = normalizeDecodedUrl(queryTarget);
      if (
        normalizedQuery.startsWith('http://') ||
        normalizedQuery.startsWith('https://') ||
        normalizedQuery.startsWith('http:/') ||
        normalizedQuery.startsWith('https:/')
      ) {
        const canonicalPath = targetPathForUrl(normalizedQuery);
        if (window.location.pathname !== canonicalPath) {
          window.location.replace(canonicalPath);
          return null;
        }
      }
      return normalizedQuery;
    }

    const path = window.location.pathname || '/';
    if (path === '/' || path === '/index.html') {
      return '';
    }

    const rawTarget = path.startsWith('/') ? path.slice(1) : path;
    if (!rawTarget) {
      return '';
    }

    let decoded = rawTarget;
    try {
      decoded = decodeURIComponent(rawTarget);
    } catch (_) {
      decoded = rawTarget;
    }
    const normalized = normalizeDecodedUrl(decoded);

    if (
      normalized.startsWith('http://') ||
      normalized.startsWith('https://') ||
      normalized.startsWith('http:/') ||
      normalized.startsWith('https:/')
    ) {
      const canonicalPath = targetPathForUrl(normalized);
      if (path !== canonicalPath) {
        window.location.replace(canonicalPath);
        return null;
      }
      return normalized;
    }

    return '';
  }

  function chapterHash(sourceUrl) {
    const input = normalizeDecodedUrl(sourceUrl);
    let hash = 2166136261;
    for (let i = 0; i < input.length; i += 1) {
      hash ^= input.charCodeAt(i);
      hash = Math.imul(hash, 16777619);
    }
    return `raw-${(hash >>> 0).toString(16).padStart(8, '0')}`;
  }

  function openDb() {
    return new Promise((resolve, reject) => {
      const req = indexedDB.open(DB_NAME, DB_VERSION);
      req.onupgradeneeded = () => {
        const db = req.result;
        let store = null;
        if (!db.objectStoreNames.contains(STORE_NAME)) {
          store = db.createObjectStore(STORE_NAME, { keyPath: 'chapter_hash' });
        } else {
          store = req.transaction.objectStore(STORE_NAME);
        }
        if (store && !store.indexNames.contains('source_key_idx')) {
          store.createIndex('source_key_idx', 'source_key', { unique: false });
        }
        if (store && !store.indexNames.contains('cached_at_idx')) {
          store.createIndex('cached_at_idx', 'cached_at', { unique: false });
        }
      };
      req.onsuccess = () => resolve(req.result);
      req.onerror = () => reject(req.error);
    });
  }

  async function idbPutChapter(entry) {
    const db = await openDb();
    const tx = db.transaction(STORE_NAME, 'readwrite');
    const store = tx.objectStore(STORE_NAME);
    store.put(entry);
    await new Promise((resolve, reject) => {
      tx.oncomplete = resolve;
      tx.onerror = () => reject(tx.error);
      tx.onabort = () => reject(tx.error);
    });
  }

  async function idbGetChapterBySourceUrl(sourceUrl) {
    const sourceKey = normalizeSourceUrlKey(sourceUrl);
    if (!sourceKey) return null;

    const db = await openDb();
    const tx = db.transaction(STORE_NAME, 'readonly');
    const store = tx.objectStore(STORE_NAME);
    const getAllReq = store.getAll();
    const rows = await new Promise((resolve, reject) => {
      getAllReq.onsuccess = () => resolve(getAllReq.result || []);
      getAllReq.onerror = () => reject(getAllReq.error);
    });

    const matched = rows.filter((row) => row && row.source_key === sourceKey);
    matched.sort((a, b) => (b.cached_at || 0) - (a.cached_at || 0));

    const ready = matched.find((row) => Array.isArray(row.images) && row.images.length > 0);
    return ready || matched[0] || null;
  }

  async function idbTrimMax20() {
    const db = await openDb();
    const tx = db.transaction(STORE_NAME, 'readwrite');
    const store = tx.objectStore(STORE_NAME);
    const req = store.getAll();
    const all = await new Promise((resolve, reject) => {
      req.onsuccess = () => resolve(req.result || []);
      req.onerror = () => reject(req.error);
    });

    all.sort((a, b) => (a.cached_at || 0) - (b.cached_at || 0));
    while (all.length > MAX_CACHED_CHAPTERS) {
      const oldest = all.shift();
      if (oldest && oldest.chapter_hash) {
        store.delete(oldest.chapter_hash);
      }
    }

    await new Promise((resolve, reject) => {
      tx.oncomplete = resolve;
      tx.onerror = () => reject(tx.error);
      tx.onabort = () => reject(tx.error);
    });
  }

  async function fetchTextWithFallback(url) {
    let lastError = null;
    for (const build of FETCH_BUILDERS) {
      const attempt = build(url);
      if (isAttemptCoolingDown(attempt)) {
        continue;
      }
      try {
        const text = await withInflight(state.inflightTextRequests, attempt, () =>
          fetchTextAttempt(attempt)
        );
        return text;
      } catch (err) {
        lastError = err;
      }
    }
    throw (
      lastError ||
      new Error(
        'failed to fetch chapter html via proxy endpoint. make sure EdgeOne Functions is enabled.'
      )
    );
  }

  async function fetchBlobWithFallback(url) {
    for (const build of FETCH_BUILDERS) {
      const attempt = build(url);
      if (isAttemptCoolingDown(attempt)) {
        continue;
      }
      try {
        const blob = await withInflight(state.inflightBlobRequests, attempt, () =>
          fetchBlobAttempt(attempt)
        );
        return blob;
      } catch (_) {
        // continue
      }
    }
    return null;
  }

  function resolveAbsoluteUrl(baseUrl, rawUrl) {
    try {
      return new URL(rawUrl, baseUrl).href;
    } catch (_) {
      return null;
    }
  }

  function extractTitle(doc) {
    const selectors = ['div header h1', 'h1'];
    for (const selector of selectors) {
      const element = doc.querySelector(selector);
      if (!element) continue;
      const text = element.textContent ? element.textContent.trim() : '';
      if (text) return text;
    }
    return 'Manga Chapter';
  }

  function extractImageUrls(doc, sourceUrl) {
    const selectors = ['#Baca_Komik img', 'div#Baca_Komik img', 'img'];
    const seen = new Set();
    const output = [];

    for (const selector of selectors) {
      const nodes = Array.from(doc.querySelectorAll(selector));
      for (const node of nodes) {
        const raw =
          (node.getAttribute('src') ||
            node.getAttribute('data-src') ||
            node.getAttribute('data-lazy-src') ||
            '')
            .trim();

        if (!raw || raw.startsWith('data:')) continue;
        const absolute = resolveAbsoluteUrl(sourceUrl, raw);
        if (!absolute || seen.has(absolute)) continue;
        seen.add(absolute);
        output.push(absolute);
      }
      if (output.length > 0) {
        return output;
      }
    }

    return output;
  }

  function extractChapterNumber(pathname) {
    const lower = String(pathname || '').toLowerCase();
    const marker = 'chapter-';
    const markerIndex = lower.indexOf(marker);
    if (markerIndex < 0) return null;
    let digits = '';
    const start = markerIndex + marker.length;
    for (let i = start; i < lower.length; i += 1) {
      const ch = lower[i];
      if (ch >= '0' && ch <= '9') {
        digits += ch;
      } else {
        break;
      }
    }
    if (!digits) return null;
    const value = Number(digits);
    return Number.isFinite(value) ? value : null;
  }

  function extractNextUrl(doc, sourceUrl) {
    const selectors = [
      "a[rel='next']",
      'a.next',
      '.next a',
      '.navig a',
      '.pagination a',
      'a',
    ];

    const candidates = [];
    let currentChapter = null;
    try {
      currentChapter = extractChapterNumber(new URL(sourceUrl).pathname);
    } catch (_) {
      currentChapter = null;
    }

    for (const selector of selectors) {
      const nodes = Array.from(doc.querySelectorAll(selector));
      for (const node of nodes) {
        const href = (node.getAttribute('href') || '').trim();
        if (!href || href === '#' || href.startsWith('javascript:')) continue;

        const absolute = resolveAbsoluteUrl(sourceUrl, href);
        if (!absolute || absolute === sourceUrl) continue;

        const rel = (node.getAttribute('rel') || '').toLowerCase();
        const text = (node.textContent || '').trim().toLowerCase();

        if (
          rel.includes('next') ||
          text.includes('next') ||
          text.includes('selanjutnya')
        ) {
          return absolute;
        }

        candidates.push(absolute);
      }
      if (candidates.length > 0) break;
    }

    if (currentChapter !== null) {
      const wanted = `chapter-${currentChapter + 1}`;
      const exact = candidates.find((candidate) =>
        candidate.toLowerCase().includes(wanted)
      );
      if (exact) return exact;
    }

    const chapterCandidates = candidates.filter((candidate) =>
      candidate.toLowerCase().includes('chapter-')
    );

    return chapterCandidates.length > 0
      ? chapterCandidates[chapterCandidates.length - 1]
      : null;
  }

  async function fetchChapter(sourceUrl) {
    const sourceKey = normalizeSourceUrlKey(sourceUrl);
    const cachedManifest = state.chapterManifestCache.get(sourceKey);
    if (cachedManifest) {
      return cloneChapterManifest(cachedManifest);
    }

    const html = await fetchTextWithFallback(sourceUrl);
    const doc = new DOMParser().parseFromString(html, 'text/html');
    const title = extractTitle(doc);
    const imageUrls = extractImageUrls(doc, sourceUrl);
    if (imageUrls.length === 0) {
      throw new Error('image extraction returned zero URLs');
    }
    const nextUrl = extractNextUrl(doc, sourceUrl);

    const chapter = {
      sourceUrl,
      sourceKey,
      chapterHash: chapterHash(sourceUrl),
      title,
      imageUrls,
      nextUrl,
    };
    cacheChapterManifest(chapter);
    return cloneChapterManifest(chapter);
  }

  async function renderImagesSequential(items, token) {
    const elements = [];
    for (let i = 0; i < items.length; i += 1) {
      const img = document.createElement('img');
      img.className = 'page';
      img.alt = `Page ${i + 1}`;
      img.decoding = 'async';
      pagesEl.appendChild(img);
      elements.push(img);
    }

    for (let i = 0; i < items.length; i += 1) {
      if (token !== state.loadToken) return false;
      await new Promise((resolve) => {
        const img = elements[i];
        const item = items[i];
        const primary = typeof item === 'string' ? item : item && item.src ? item.src : '';
        const fallback =
          typeof item === 'object' && item && item.fallback ? item.fallback : null;
        let fallbackTried = false;

        img.onload = resolve;
        img.onerror = () => {
          if (fallback && !fallbackTried) {
            fallbackTried = true;
            img.src = fallback;
            return;
          }
          resolve();
        };

        if (!primary) {
          resolve();
          return;
        }
        img.src = primary;
      });
    }

    return true;
  }

  async function renderChapterFromNetwork(chapter, token) {
    resetPages();
    document.title = chapter.title;
    chapterTitleEl.textContent = chapter.title;
    setNextButton(chapter.nextUrl);
    setMeta(`Loading ${chapter.imageUrls.length} images sequentially from raw source...`);

    // Reduce Edge Function traffic: try direct upstream image first, proxy only on failure.
    const displayUrls = chapter.imageUrls.map((url) => ({
      src: url,
      fallback: proxyUrlFor(url),
    }));
    const finished = await renderImagesSequential(displayUrls, token);
    if (!finished) return;

    setMeta('Chapter loaded. Background prefetch to IndexedDB is running...');
    requestPrefetch(chapter.nextUrl);
  }

  async function renderChapterFromCache(cached, token) {
    const blobs = (cached.images || []).filter((item) => item && item.blob);
    if (blobs.length === 0) {
      return false;
    }

    resetPages();
    document.title = cached.title || 'Raw Manga Chapter';
    chapterTitleEl.textContent = cached.title || 'Raw Manga Chapter';
    setNextButton(cached.next_url || null);

    const urls = [];
    for (const item of blobs) {
      const objectUrl = URL.createObjectURL(item.blob);
      state.activeObjectUrls.push(objectUrl);
      urls.push(objectUrl);
    }

    setMeta(`Loaded ${urls.length} images from local IndexedDB cache.`);
    const finished = await renderImagesSequential(urls, token);
    if (!finished) return false;

    requestPrefetch(cached.next_url || null);
    return true;
  }

  async function loadChapter(sourceUrl, preferLocal) {
    const normalized = normalizeDecodedUrl(sourceUrl);
    if (!normalized) {
      throw new Error('empty chapter url');
    }

    const token = ++state.loadToken;
    showReader(normalized);

    if (preferLocal) {
      const cached = await idbGetChapterBySourceUrl(normalized);
      if (cached && (cached.images || []).length > 0) {
        const rendered = await renderChapterFromCache(cached, token);
        if (rendered) {
          return;
        }
      }
    }

    setMeta('Fetching chapter HTML...');
    const chapter = await fetchChapter(normalized);
    if (token !== state.loadToken) return;

    await renderChapterFromNetwork(chapter, token);
  }

  async function savePrefetchedChapter(chapter) {
    const images = [];

    for (const imageUrl of chapter.imageUrls) {
      const blob = await fetchBlobWithFallback(imageUrl);
      if (!blob) continue;
      images.push({ url: imageUrl, blob });
    }

    if (images.length === 0) return;

    await idbPutChapter({
      chapter_hash: chapter.chapterHash,
      source_url: chapter.sourceUrl,
      source_key: chapter.sourceKey,
      title: chapter.title,
      next_url: chapter.nextUrl,
      images,
      cached_at: Date.now(),
    });

    await idbTrimMax20();
  }

  async function prefetchNextChapters(startUrl, depth) {
    let nextUrl = normalizeDecodedUrl(startUrl);

    for (let i = 0; i < depth; i += 1) {
      if (!nextUrl) break;

      const cached = await idbGetChapterBySourceUrl(nextUrl);
      if (cached && Array.isArray(cached.images) && cached.images.length > 0) {
        nextUrl = normalizeDecodedUrl(cached.next_url || '');
        continue;
      }

      let chapter = null;
      try {
        chapter = await fetchChapter(nextUrl);
      } catch (_) {
        break;
      }

      try {
        await savePrefetchedChapter(chapter);
      } catch (_) {
        // continue to next if possible
      }

      nextUrl = normalizeDecodedUrl(chapter.nextUrl || '');
    }
  }

  function requestPrefetch(startUrl) {
    const normalized = normalizeDecodedUrl(startUrl);
    if (!normalized) return;
    state.pendingPrefetchStart = normalized;
    if (!state.prefetchRunning) {
      void runPrefetchLoop();
    }
  }

  async function runPrefetchLoop() {
    state.prefetchRunning = true;
    try {
      while (state.pendingPrefetchStart) {
        const start = state.pendingPrefetchStart;
        state.pendingPrefetchStart = null;
        await prefetchNextChapters(start, PREFETCH_DEPTH);
      }
    } finally {
      state.prefetchRunning = false;
    }
  }

  function bindEvents() {
    openForm.addEventListener('submit', (event) => {
      event.preventDefault();
      const raw = normalizeDecodedUrl(targetInput.value);
      if (!raw) return;
      window.location.href = targetPathForUrl(raw);
    });
  }

  async function bootstrap() {
    bindEvents();
    const target = getInitialTargetFromLocation();
    if (target === null) {
      return;
    }
    if (!target) {
      showInput();
      return;
    }

    try {
      await loadChapter(target, true);
    } catch (err) {
      showReader(target);
      setNextButton(null);
      const message = err && err.message ? err.message : String(err);
      setMeta(`Failed to load chapter: ${message}`);
    }
  }

  void bootstrap();
})();
