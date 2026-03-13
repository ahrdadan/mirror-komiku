<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import Reader from "./components/reader/Reader.svelte";
  import SettingsPanel from "./components/settings/SettingsPanel.svelte";
  import Toast from "./components/ui/Toast.svelte";
  import {
    clearAllCache,
    enforceLruLimit,
    getCacheCount,
    getCachedChapter,
    type CachedImageBlob,
    normalizeChapterUrl,
    upsertChapter
  } from "./db/cache";
  import { decodeBase64Url, encodeBase64Url } from "./lib/base64url";
  import { parsePath } from "./lib/routing";
  import {
    canonicalPath,
    findProviderById,
    matchProviderByUrl
  } from "./providers";
  import type { ParsedChapter, ProviderParser } from "./providers/types";

  const CACHE_LIMIT = 10;
  const DEFAULT_PREFETCH_DEPTH = 3;

  interface DisplayChapter {
    provider: ProviderParser;
    sourceUrl: string;
    title: string;
    imageUrls: string[];
    displayImages: string[];
    nextUrl: string | null;
    fromCache: boolean;
  }

  let inputUrl = "";
  let loading = false;
  let status = "Paste chapter URL and start reading.";
  let currentChapter: DisplayChapter | null = null;
  let currentPageIndex = 0;
  let settingsOpen = false;
  let cacheCount = 0;
  let prefetchDepth = loadPrefetchDepth();
  let cacheImages = loadCacheImagesFlag();
  let toastVisible = false;
  let toastMessage = "";
  let toastTimeout: ReturnType<typeof setTimeout> | null = null;
  const memoryManifest = new Map<string, ParsedChapter>();
  const activeObjectUrls: string[] = [];
  let prefetchChain = Promise.resolve();

  $: prefetchDepth = clamp(prefetchDepth, 1, 5);
  $: localStorage.setItem("mirror_prefetch_depth", String(prefetchDepth));
  $: localStorage.setItem("mirror_cache_images", String(cacheImages));

  onMount(() => {
    void refreshCacheCount();
    void syncFromLocation();
    const onPopState = () => {
      void syncFromLocation();
    };
    window.addEventListener("popstate", onPopState);
    return () => {
      window.removeEventListener("popstate", onPopState);
    };
  });

  onDestroy(() => {
    releaseObjectUrls();
    if (toastTimeout) clearTimeout(toastTimeout);
  });

  function loadPrefetchDepth(): number {
    const stored = Number(localStorage.getItem("mirror_prefetch_depth"));
    if (!Number.isFinite(stored)) return DEFAULT_PREFETCH_DEPTH;
    return clamp(stored, 1, 5);
  }

  function loadCacheImagesFlag(): boolean {
    const stored = localStorage.getItem("mirror_cache_images");
    if (!stored) return true;
    return stored !== "false";
  }

  function clamp(value: number, min: number, max: number): number {
    if (value < min) return min;
    if (value > max) return max;
    return value;
  }

  function showToast(message: string) {
    toastMessage = message;
    toastVisible = true;
    if (toastTimeout) clearTimeout(toastTimeout);
    toastTimeout = setTimeout(() => {
      toastVisible = false;
    }, 2800);
  }

  function normalizeInputUrl(value: string): string {
    const trimmed = value.trim();
    if (!trimmed) return "";
    if (trimmed.startsWith("https:/") && !trimmed.startsWith("https://")) {
      return trimmed.replace("https:/", "https://");
    }
    if (trimmed.startsWith("http:/") && !trimmed.startsWith("http://")) {
      return trimmed.replace("http:/", "http://");
    }
    return trimmed;
  }

  async function refreshCacheCount() {
    cacheCount = await getCacheCount();
  }

  function releaseObjectUrls() {
    while (activeObjectUrls.length > 0) {
      const next = activeObjectUrls.pop();
      if (!next) continue;
      URL.revokeObjectURL(next);
    }
  }

  async function fetchHtml(provider: ProviderParser, sourceUrl: string): Promise<string> {
    const encoded = encodeBase64Url(sourceUrl);
    const response = await fetch(`/api/proxy/${provider.id}/${encoded}`, {
      method: "GET",
      credentials: "omit"
    });
    if (!response.ok) {
      throw new Error(`proxy status ${response.status}`);
    }
    return response.text();
  }

  async function fetchParsedChapter(
    provider: ProviderParser,
    sourceUrl: string
  ): Promise<ParsedChapter> {
    const normalized = normalizeChapterUrl(sourceUrl);
    const cacheKey = `${provider.id}:${normalized}`;
    const mem = memoryManifest.get(cacheKey);
    if (mem) return mem;

    const html = await fetchHtml(provider, normalized);
    const parsed = provider.parseChapter(html, normalized);
    memoryManifest.set(cacheKey, parsed);
    return parsed;
  }

  async function toDisplayFromCache(
    provider: ProviderParser,
    chapterUrl: string
  ): Promise<DisplayChapter | null> {
    const cached = await getCachedChapter(provider.id, chapterUrl);
    if (!cached) return null;

    let displayImages: string[] = [...cached.images];
    if (cached.imageBlobs && cached.imageBlobs.length > 0) {
      releaseObjectUrls();
      displayImages = cached.imageBlobs.map((item) => {
        const objectUrl = URL.createObjectURL(item.blob);
        activeObjectUrls.push(objectUrl);
        return objectUrl;
      });
    }

    return {
      provider,
      sourceUrl: cached.chapterUrl,
      title: cached.title,
      imageUrls: [...cached.images],
      displayImages,
      nextUrl: cached.nextUrl,
      fromCache: true
    };
  }

  async function saveToCache(
    provider: ProviderParser,
    chapter: ParsedChapter,
    imageBlobs?: CachedImageBlob[]
  ) {
    await upsertChapter({
      providerId: provider.id,
      chapterUrl: chapter.sourceUrl,
      title: chapter.title,
      images: chapter.imageUrls,
      nextUrl: chapter.nextUrl,
      imageBlobs
    });
    await enforceLruLimit(CACHE_LIMIT);
    await refreshCacheCount();
  }

  async function loadChapter(
    provider: ProviderParser,
    chapterUrl: string,
    preferCache: boolean
  ) {
    loading = true;
    status = "Loading chapter...";
    const normalized = normalizeChapterUrl(chapterUrl);

    try {
      if (preferCache) {
        const fromCache = await toDisplayFromCache(provider, normalized);
        if (fromCache) {
          currentChapter = fromCache;
          currentPageIndex = 0;
          status = `Loaded from cache: ${fromCache.title}`;
          requestPrefetch(provider, fromCache.nextUrl);
          return;
        }
      }

      releaseObjectUrls();
      const parsed = await fetchParsedChapter(provider, normalized);
      currentChapter = {
        provider,
        sourceUrl: parsed.sourceUrl,
        title: parsed.title,
        imageUrls: [...parsed.imageUrls],
        displayImages: [...parsed.imageUrls],
        nextUrl: parsed.nextUrl,
        fromCache: false
      };
      currentPageIndex = 0;
      status = `Loaded ${parsed.imageUrls.length} pages from network.`;
      await saveToCache(provider, parsed);
      requestPrefetch(provider, parsed.nextUrl);
    } finally {
      loading = false;
    }
  }

  async function fetchBlob(url: string): Promise<Blob | null> {
    try {
      const response = await fetch(url, { method: "GET", credentials: "omit" });
      if (!response.ok) return null;
      const blob = await response.blob();
      if (!blob || blob.size === 0) return null;
      return blob;
    } catch {
      return null;
    }
  }

  function buildPrefetchCandidates(startUrl: string, depth: number): string[] {
    const normalizedStart = normalizeChapterUrl(startUrl);
    const out = [normalizedStart];
    const seen = new Set(out);
    const match = normalizedStart.match(/chapter-(\d+)/i);
    if (!match) return out;

    const current = Number(match[1]);
    if (!Number.isFinite(current)) return out;

    for (let i = 1; i < depth; i += 1) {
      const candidate = normalizedStart.replace(
        /chapter-(\d+)/i,
        `chapter-${current + i}`
      );
      if (seen.has(candidate)) continue;
      seen.add(candidate);
      out.push(candidate);
    }

    return out;
  }

  async function prefetchOne(provider: ProviderParser, chapterUrl: string) {
    const normalized = normalizeChapterUrl(chapterUrl);
    const cached = await getCachedChapter(provider.id, normalized);
    if (cached && (!cacheImages || (cached.imageBlobs?.length ?? 0) > 0)) {
      return;
    }

    try {
      const parsed = await fetchParsedChapter(provider, normalized);
      if (!cacheImages) {
        await saveToCache(provider, parsed);
        return;
      }

      const blobResults = await Promise.allSettled(
        parsed.imageUrls.map(async (imageUrl) => {
          const blob = await fetchBlob(imageUrl);
          return blob ? { url: imageUrl, blob } : null;
        })
      );
      const imageBlobs = blobResults
        .filter((item): item is PromiseFulfilledResult<CachedImageBlob | null> => {
          return item.status === "fulfilled";
        })
        .map((item) => item.value)
        .filter((item): item is CachedImageBlob => item !== null);

      await saveToCache(provider, parsed, imageBlobs.length > 0 ? imageBlobs : undefined);
    } catch {
      // Prefetch should not block reader interaction.
    }
  }

  function requestPrefetch(provider: ProviderParser, startUrl: string | null) {
    if (!startUrl || prefetchDepth <= 0) return;
    const candidates = buildPrefetchCandidates(startUrl, prefetchDepth);
    prefetchChain = prefetchChain
      .then(async () => {
        await Promise.allSettled(candidates.map((url) => prefetchOne(provider, url)));
      })
      .catch(() => {
        // Ignore queue errors and keep chain alive.
      });
  }

  async function navigateToChapterUrl(rawUrl: string, replace = false) {
    const normalized = normalizeInputUrl(rawUrl);
    const provider = matchProviderByUrl(normalized);
    if (!provider) {
      throw new Error("unsupported provider domain");
    }
    const normalizedUrl = normalizeChapterUrl(normalized);
    const path = canonicalPath(provider.id, encodeBase64Url(normalizedUrl));
    if (replace) {
      history.replaceState({}, "", path);
    } else {
      history.pushState({}, "", path);
    }
    await loadChapter(provider, normalizedUrl, true);
  }

  async function handleCurrentLocation() {
    const route = parsePath(window.location.pathname);
    if (route.kind === "home") {
      currentChapter = null;
      currentPageIndex = 0;
      status = "Paste chapter URL and start reading.";
      return;
    }

    if (route.kind === "raw-url") {
      await navigateToChapterUrl(route.rawUrl, true);
      return;
    }

    const provider = findProviderById(route.providerId);
    if (!provider) {
      throw new Error(`unknown provider '${route.providerId}'`);
    }

    let decoded = "";
    try {
      decoded = decodeBase64Url(route.encoded);
    } catch {
      throw new Error("invalid encoded chapter url");
    }

    const normalized = normalizeChapterUrl(decoded);
    const matchedProvider = matchProviderByUrl(normalized);
    if (!matchedProvider || matchedProvider.id !== provider.id) {
      throw new Error("decoded url does not match provider domain");
    }

    await loadChapter(provider, normalized, true);
  }

  async function syncFromLocation() {
    try {
      await handleCurrentLocation();
    } catch (error) {
      loading = false;
      const message = error instanceof Error ? error.message : String(error);
      status = `Route error: ${message}`;
      showToast(`Route error: ${message}`);
    }
  }

  async function submitOpenUrl(event: Event) {
    event.preventDefault();
    if (!inputUrl.trim()) return;
    try {
      await navigateToChapterUrl(inputUrl, false);
      inputUrl = "";
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      showToast(`Open URL failed: ${message}`);
    }
  }

  async function openNextChapter() {
    if (!currentChapter?.nextUrl) {
      showToast("No next chapter URL detected");
      return;
    }
    try {
      await navigateToChapterUrl(currentChapter.nextUrl, false);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      showToast(`Next chapter failed: ${message}`);
    }
  }

  async function clearCache() {
    await clearAllCache();
    await refreshCacheCount();
    showToast("IndexedDB cache cleared");
  }

  function handlePrefetchDepthChange(event: CustomEvent<number>) {
    prefetchDepth = clamp(event.detail, 1, 5);
  }

  function handleCacheImagesChange(event: CustomEvent<boolean>) {
    cacheImages = event.detail;
  }
</script>

<main class="layout">
  <header class="topbar">
    <div>
      <p class="brand">Mirror Komiku v3</p>
      <p class="status">{loading ? "Loading..." : status}</p>
    </div>
    <button type="button" class="secondary" on:click={() => (settingsOpen = true)}>Settings</button>
  </header>

  <section class="card">
    <form class="open-form" on:submit={submitOpenUrl}>
      <input
        type="url"
        placeholder="https://komiku.org/martial-peak-chapter-981/"
        bind:value={inputUrl}
      />
      <button type="submit">Open</button>
    </form>
  </section>

  {#if currentChapter}
    <section class="card">
      <Reader
        title={currentChapter.title}
        images={currentChapter.displayImages}
        bind:currentIndex={currentPageIndex}
        on:requestnext={openNextChapter}
      />
      {#if currentChapter.nextUrl}
        <div class="next-wrap">
          <button type="button" on:click={openNextChapter}>Go To Next Chapter</button>
        </div>
      {/if}
    </section>
  {/if}
</main>

<SettingsPanel
  open={settingsOpen}
  prefetchDepth={prefetchDepth}
  cacheImages={cacheImages}
  cacheCount={cacheCount}
  cacheLimit={CACHE_LIMIT}
  on:close={() => (settingsOpen = false)}
  on:clearcache={clearCache}
  on:prefetchchange={handlePrefetchDepthChange}
  on:cacheimageschange={handleCacheImagesChange}
/>

<Toast visible={toastVisible} message={toastMessage} />

<style>
  .layout {
    width: min(1080px, 95vw);
    margin: 0 auto;
    padding: 1rem 0 2rem;
    display: grid;
    gap: 0.9rem;
  }

  .topbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
  }

  .brand {
    margin: 0;
    font-size: 1.2rem;
    font-weight: 700;
  }

  .status {
    margin: 0.2rem 0 0;
    color: #475569;
    font-size: 0.92rem;
  }

  .card {
    background: #ffffff;
    border: 1px solid #dbe2ea;
    border-radius: 0.9rem;
    padding: 0.9rem;
    box-shadow: 0 8px 20px rgb(15 23 42 / 0.06);
  }

  .open-form {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0.6rem;
  }

  .open-form input {
    border: 1px solid #cbd5e1;
    border-radius: 0.6rem;
    padding: 0.55rem 0.65rem;
    width: 100%;
  }

  button {
    border: 1px solid #2563eb;
    background: #2563eb;
    color: #ffffff;
    border-radius: 0.6rem;
    padding: 0.5rem 0.8rem;
    cursor: pointer;
  }

  .secondary {
    border-color: #cbd5e1;
    background: #ffffff;
    color: #111827;
  }

  .next-wrap {
    margin-top: 0.8rem;
    display: flex;
    justify-content: flex-end;
  }

  @media (max-width: 768px) {
    .open-form {
      grid-template-columns: 1fr;
    }
  }
</style>
