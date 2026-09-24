<script lang="ts">
  // Panel sliding in from the right edge, with a title and a close button.
  import type { Snippet } from "svelte";
  import { tr } from "$lib/prefs.svelte";
  let { title, label, onclose, children }: { title: string; label: string; onclose: () => void; children: Snippet } = $props();
</script>

<aside aria-label={label}>
  <button class="close" onclick={onclose} aria-label={tr("common.close")}>×</button>
  <h2>{title}</h2>
  {@render children()}
</aside>

<style>
  aside {
    position: fixed; top: 0; right: 0; bottom: 0; z-index: 150; width: min(440px, 100vw); box-sizing: border-box;
    padding: 24px; overflow-y: auto; background: var(--panel); border-left: 1px solid var(--border);
    box-shadow: -12px 0 32px rgba(0, 0, 0, 0.5);
  }
  h2 { margin: 0 0 14px; padding-right: 40px; }
</style>
