<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let open = false;
  export let title = "Settings";

  const dispatch = createEventDispatcher<{ close: void }>();

  function close() {
    dispatch("close");
  }
</script>

{#if open}
  <div class="sheet-root">
    <button class="sheet-backdrop" type="button" aria-label="Close panel" on:click={close}></button>
    <div class="sheet-panel" role="dialog" aria-modal="true" aria-label={title}>
      <header class="sheet-header">
        <h2>{title}</h2>
        <button class="plain-btn" type="button" on:click={close}>Close</button>
      </header>
      <section class="sheet-content">
        <slot></slot>
      </section>
    </div>
  </div>
{/if}

<style>
  .sheet-root {
    position: fixed;
    inset: 0;
    z-index: 50;
  }

  .sheet-backdrop {
    position: absolute;
    inset: 0;
    background: rgb(15 23 42 / 0.4);
    border: 0;
    width: 100%;
  }

  .sheet-panel {
    position: absolute;
    right: 0;
    top: 0;
    bottom: 0;
    width: min(420px, 90vw);
    background: #ffffff;
    border-left: 1px solid #e2e8f0;
    box-shadow: -8px 0 24px rgb(15 23 42 / 0.2);
    display: flex;
    flex-direction: column;
  }

  .sheet-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem;
    border-bottom: 1px solid #e2e8f0;
  }

  .sheet-header h2 {
    margin: 0;
    font-size: 1rem;
  }

  .plain-btn {
    border: 1px solid #cbd5e1;
    background: #fff;
    border-radius: 0.5rem;
    padding: 0.4rem 0.7rem;
    cursor: pointer;
  }

  .sheet-content {
    padding: 1rem;
    overflow: auto;
  }
</style>
