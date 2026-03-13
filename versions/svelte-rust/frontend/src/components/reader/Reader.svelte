<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let title = "Manga Chapter";
  export let images: string[] = [];
  export let currentIndex = 0;

  const dispatch = createEventDispatcher<{ requestnext: void }>();

  let viewport: HTMLDivElement | null = null;
  let dragging = false;
  let pointerId: number | null = null;
  let startX = 0;
  let dragOffset = 0;
  let unlockedCount = 0;
  let imageFingerprint = "";

  $: {
    const nextFingerprint = images.join("|");
    if (nextFingerprint !== imageFingerprint) {
      imageFingerprint = nextFingerprint;
      unlockedCount = images.length > 0 ? 1 : 0;
      currentIndex = 0;
      dragOffset = 0;
    }
  }

  $: {
    if (images.length === 0) {
      currentIndex = 0;
    } else if (currentIndex < 0) {
      currentIndex = 0;
    } else if (currentIndex >= images.length) {
      currentIndex = images.length - 1;
    }
  }

  function onImageLoaded(index: number) {
    const nextUnlock = Math.min(images.length, index + 2);
    if (nextUnlock > unlockedCount) {
      unlockedCount = nextUnlock;
    }
  }

  function previousPage() {
    if (currentIndex <= 0) return;
    currentIndex -= 1;
  }

  function nextPage() {
    if (currentIndex >= images.length - 1) {
      dispatch("requestnext");
      return;
    }
    currentIndex += 1;
  }

  function onPointerDown(event: PointerEvent) {
    if (!viewport) return;
    dragging = true;
    pointerId = event.pointerId;
    startX = event.clientX;
    dragOffset = 0;
    viewport.setPointerCapture(event.pointerId);
  }

  function onPointerMove(event: PointerEvent) {
    if (!dragging || event.pointerId !== pointerId) return;
    dragOffset = event.clientX - startX;
  }

  function onPointerUp(event: PointerEvent) {
    if (!dragging || event.pointerId !== pointerId) return;
    const threshold = 52;
    if (dragOffset > threshold) previousPage();
    if (dragOffset < -threshold) nextPage();
    dragOffset = 0;
    dragging = false;
    pointerId = null;
  }

  $: trackTransform = `translateX(calc(${-currentIndex * 100}% + ${dragOffset}px))`;
</script>

<div class="reader-shell">
  <header class="reader-header">
    <h1>{title}</h1>
    <div class="reader-meta">{images.length === 0 ? "No pages" : `${currentIndex + 1}/${images.length}`}</div>
  </header>

  <div
    class="reader-viewport"
    role="region"
    aria-label="Reader viewport"
    bind:this={viewport}
    on:pointerdown={onPointerDown}
    on:pointermove={onPointerMove}
    on:pointerup={onPointerUp}
    on:pointercancel={onPointerUp}
  >
    <div class="reader-track" style:transform={trackTransform}>
      {#each images as image, index}
        <section class="reader-slide">
          {#if index < unlockedCount}
            <img
              class="page-image"
              src={image}
              alt={`Page ${index + 1}`}
              draggable="false"
              loading={index <= currentIndex + 1 ? "eager" : "lazy"}
              on:load={() => onImageLoaded(index)}
              on:error={() => onImageLoaded(index)}
            />
          {:else}
            <div class="placeholder">Queued page {index + 1}</div>
          {/if}
        </section>
      {/each}
    </div>
  </div>

  <footer class="reader-controls">
    <button type="button" on:click={previousPage} disabled={currentIndex <= 0}>Prev</button>
    <button type="button" on:click={nextPage}>
      {currentIndex >= images.length - 1 ? "Open Next Chapter" : "Next"}
    </button>
  </footer>
</div>

<style>
  .reader-shell {
    display: grid;
    gap: 0.8rem;
  }

  .reader-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }

  .reader-header h1 {
    margin: 0;
    font-size: 1rem;
    line-height: 1.2;
  }

  .reader-meta {
    font-size: 0.87rem;
    color: #475569;
    white-space: nowrap;
  }

  .reader-viewport {
    position: relative;
    overflow: hidden;
    border-radius: 0.8rem;
    border: 1px solid #dbe2ea;
    background: #020617;
    min-height: 58vh;
    touch-action: pan-y;
  }

  .reader-track {
    display: flex;
    transition: transform 220ms ease;
    will-change: transform;
  }

  .reader-slide {
    min-width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 58vh;
  }

  .page-image {
    width: 100%;
    height: auto;
    object-fit: contain;
    user-select: none;
    pointer-events: none;
  }

  .placeholder {
    color: #cbd5e1;
    font-size: 0.9rem;
    border: 1px dashed #475569;
    padding: 1rem;
    border-radius: 0.7rem;
  }

  .reader-controls {
    display: flex;
    gap: 0.6rem;
  }

  .reader-controls button {
    border: 1px solid #cbd5e1;
    background: #fff;
    border-radius: 0.6rem;
    padding: 0.45rem 0.8rem;
    cursor: pointer;
  }

  .reader-controls button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
</style>
