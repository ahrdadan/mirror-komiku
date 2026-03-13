<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let open = false;
  export let title = "Dialog";

  const dispatch = createEventDispatcher<{ close: void }>();

  function closeDialog() {
    dispatch("close");
  }
</script>

{#if open}
  <div class="dialog-root">
    <button class="dialog-backdrop" type="button" on:click={closeDialog} aria-label="Close dialog"></button>
    <div class="dialog" role="dialog" aria-modal="true" aria-label={title}>
      <header>
        <h3>{title}</h3>
      </header>
      <section>
        <slot></slot>
      </section>
      <footer>
        <button type="button" on:click={closeDialog}>Close</button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .dialog-root {
    position: fixed;
    inset: 0;
    z-index: 55;
  }

  .dialog-backdrop {
    position: absolute;
    inset: 0;
    border: 0;
    width: 100%;
    background: rgb(15 23 42 / 0.45);
  }

  .dialog {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: min(420px, 92vw);
    background: #ffffff;
    border-radius: 0.8rem;
    border: 1px solid #e2e8f0;
    padding: 0.9rem;
    display: grid;
    gap: 0.8rem;
  }

  .dialog h3 {
    margin: 0;
  }

  .dialog footer {
    display: flex;
    justify-content: flex-end;
  }

  .dialog button {
    border: 1px solid #cbd5e1;
    background: #ffffff;
    border-radius: 0.55rem;
    padding: 0.4rem 0.75rem;
    cursor: pointer;
  }
</style>
