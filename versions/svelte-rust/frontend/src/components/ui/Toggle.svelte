<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let checked = false;
  export let label = "";

  const dispatch = createEventDispatcher<{ change: boolean }>();

  function onChange() {
    dispatch("change", checked);
  }
</script>

<label class="toggle">
  <span>{label}</span>
  <input type="checkbox" bind:checked on:change={onChange} />
</label>

<style>
  .toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.6rem 0;
  }

  .toggle span {
    font-size: 0.95rem;
  }

  .toggle input {
    width: 1.2rem;
    height: 1.2rem;
  }
</style>
