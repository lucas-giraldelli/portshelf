<script lang="ts">
  // Controller hint that works for every layout: the four face buttons as a diamond
  // with the one to press filled in (south = Xbox A / PlayStation ✕ / Nintendo B, ...).
  // Also draws the D-pad.
  let { button }: { button: "south" | "east" | "west" | "north" | "dpad" } = $props();
  const spots = { north: [10, 3], west: [3, 10], east: [17, 10], south: [10, 17] } as const;
</script>

<svg class="glyph" viewBox="0 0 20 20" width="20" height="20" aria-hidden="true">
  {#if button === "dpad"}
    <path d="M7.5 1.5h5v6h6v5h-6v6h-5v-6h-6v-5h6z" fill="currentColor" />
  {:else}
    {#each Object.entries(spots) as [name, [x, y]]}
      <circle cx={x} cy={y} r="2.6" fill={name === button ? "currentColor" : "none"} stroke="currentColor" stroke-width="1.3" />
    {/each}
  {/if}
</svg>

<style>
  .glyph { display: inline-block; vertical-align: middle; color: var(--text); }
</style>
