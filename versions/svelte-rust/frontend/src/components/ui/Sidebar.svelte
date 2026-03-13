<script lang="ts">
  export let open = false;
  export let width = "280px";
</script>

<aside class:open style={`--sidebar-width:${width};`} aria-hidden={!open}>
  <slot></slot>
</aside>

<style>
  aside {
    position: fixed;
    left: 0;
    top: 0;
    bottom: 0;
    width: var(--sidebar-width);
    transform: translateX(-100%);
    transition: transform 180ms ease;
    background: #ffffff;
    border-right: 1px solid #e2e8f0;
    box-shadow: 6px 0 20px rgb(15 23 42 / 0.12);
    z-index: 45;
    overflow: auto;
  }

  aside.open {
    transform: translateX(0);
  }
</style>
