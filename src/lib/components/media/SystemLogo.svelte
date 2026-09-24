<script lang="ts">
  // A system's logo drawn in the current text colour (the image is used as a mask), so it
  // fits every theme. Falls back to the name when there is no logo.
  import { logoAspect } from "$lib/logos";
  let { id, name, height = 40 }: { id: string; name: string; height?: number } = $props();
  let aspect = $derived(logoAspect[id]);
  // Very wide logos are capped in width and very compact ones get a little more height.
  let h = $derived(aspect ? Math.min(height * (aspect < 2 ? 1.35 : 1), (height * 7.5) / Math.max(aspect, 1)) : height);
</script>

{#if aspect}
  <span class="logo" role="img" aria-label={name} style="height:{h}px; width:{h * aspect}px; --src:url(/logos/{id}.png)"></span>
{:else}
  <span class="name">{name}</span>
{/if}

<style>
  .logo {
    display: inline-block;
    background: currentColor;
    -webkit-mask: var(--src) center / contain no-repeat;
    mask: var(--src) center / contain no-repeat;
    vertical-align: middle;
  }
</style>
