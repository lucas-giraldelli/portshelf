<script lang="ts">
  // The trophy used for achievements (the same drawing as the card shown over games), or a
  // padlock for locked ones. Drawn in the current text colour.
  let { locked = false, size = 26 }: { locked?: boolean; size?: number } = $props();
</script>

<svg viewBox="0 0 24 24" width={size} height={size} aria-hidden="true">
  {#if locked}
    <rect x="5" y="11" width="14" height="10" rx="2" />
    <path d="M8 11V8a4 4 0 0 1 8 0v3" />
  {:else}
    <path d="M7 4h10v5q0 4.5-5 5q-5-.5-5-5z" />
    <path d="M7 6q-4 0-4 2.5T7.5 11M17 6q4 0 4 2.5T16.5 11M12 14v3.5M8 20h8M9.5 17.5h5" />
  {/if}
</svg>

<style>
  svg { fill: none; stroke: currentColor; stroke-width: 1.9; stroke-linecap: round; stroke-linejoin: round; }
</style>
