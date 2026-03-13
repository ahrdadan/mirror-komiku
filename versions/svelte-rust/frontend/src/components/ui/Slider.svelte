<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let min = 1;
  export let max = 5;
  export let step = 1;
  export let value = 3;
  export let label = "";

  const dispatch = createEventDispatcher<{ change: number }>();

  function onInput(event: Event) {
    const target = event.target as HTMLInputElement;
    value = Number(target.value);
    dispatch("change", value);
  }
</script>

<label class="slider">
  <span>{label}: <strong>{value}</strong></span>
  <input type="range" {min} {max} {step} bind:value on:input={onInput} />
</label>

<style>
  .slider {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.6rem 0;
  }

  .slider span {
    font-size: 0.95rem;
  }
</style>
