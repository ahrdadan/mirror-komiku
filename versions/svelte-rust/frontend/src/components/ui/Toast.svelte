<script lang="ts">
  export let message = "";
  export let visible = false;
</script>

{#if visible}
  <div class="toast" role="status" aria-live="polite">
    {message}
  </div>
{/if}

<style>
  .toast {
    position: fixed;
    left: 50%;
    transform: translateX(-50%);
    bottom: 1.25rem;
    z-index: 60;
    background: #111827;
    color: #f8fafc;
    padding: 0.7rem 1rem;
    border-radius: 0.7rem;
    box-shadow: 0 10px 24px rgb(15 23 42 / 0.35);
    max-width: 88vw;
  }
</style>
