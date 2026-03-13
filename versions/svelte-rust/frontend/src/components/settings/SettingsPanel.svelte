<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import Sheet from "../ui/Sheet.svelte";
  import Slider from "../ui/Slider.svelte";
  import Toggle from "../ui/Toggle.svelte";

  export let open = false;
  export let prefetchDepth = 3;
  export let cacheImages = true;
  export let cacheCount = 0;
  export let cacheLimit = 10;

  const dispatch = createEventDispatcher<{
    close: void;
    clearcache: void;
    prefetchchange: number;
    cacheimageschange: boolean;
  }>();

  function closePanel() {
    dispatch("close");
  }

  function clearCache() {
    dispatch("clearcache");
  }

  function onPrefetchChange(event: CustomEvent<number>) {
    dispatch("prefetchchange", event.detail);
  }

  function onCacheImagesChange(event: CustomEvent<boolean>) {
    dispatch("cacheimageschange", event.detail);
  }
</script>

<Sheet title="Reader Settings" {open} on:close={closePanel}>
  <div class="grid">
    <p class="hint">Cache: {cacheCount}/{cacheLimit} chapter(s)</p>
    <Slider
      label="Prefetch depth"
      min={1}
      max={5}
      step={1}
      value={prefetchDepth}
      on:change={onPrefetchChange}
    />
    <Toggle
      label="Cache images as blobs"
      checked={cacheImages}
      on:change={onCacheImagesChange}
    />
    <button class="danger" type="button" on:click={clearCache}>Clear IndexedDB Cache</button>
  </div>
</Sheet>

<style>
  .grid {
    display: grid;
    gap: 0.6rem;
  }

  .hint {
    margin: 0;
    color: #475569;
  }

  .danger {
    border: 1px solid #dc2626;
    background: #fff1f2;
    color: #991b1b;
    border-radius: 0.6rem;
    padding: 0.55rem 0.8rem;
    cursor: pointer;
    margin-top: 0.6rem;
  }
</style>
