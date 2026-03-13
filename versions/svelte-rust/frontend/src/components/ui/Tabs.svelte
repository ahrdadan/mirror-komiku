<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let tabs: string[] = [];
  export let activeTab = "";

  const dispatch = createEventDispatcher<{ change: string }>();

  function setTab(tab: string) {
    activeTab = tab;
    dispatch("change", tab);
  }
</script>

<div class="tabs">
  {#each tabs as tab}
    <button
      type="button"
      class:active={tab === activeTab}
      on:click={() => setTab(tab)}
    >
      {tab}
    </button>
  {/each}
</div>
<div class="tab-content">
  <slot activeTab={activeTab}></slot>
</div>

<style>
  .tabs {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .tabs button {
    border: 1px solid #cbd5e1;
    background: #ffffff;
    border-radius: 0.55rem;
    padding: 0.35rem 0.7rem;
    cursor: pointer;
  }

  .tabs button.active {
    border-color: #2563eb;
    background: #dbeafe;
  }

  .tab-content {
    margin-top: 0.8rem;
  }
</style>
