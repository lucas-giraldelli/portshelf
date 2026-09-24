<script lang="ts">
  // One port in a list: cover thumbnail, name, and a line with its system or kind.
  import type { Snippet } from "svelte";
  import { tr } from "$lib/prefs.svelte";
  import { shelf } from "$lib/shelf.svelte";
  import type { Port } from "$lib/types";

  let { port, meta, children }: { port: Port; meta?: string; children?: Snippet } = $props();
</script>

<span class="port-row">
  <span class="thumb">{#if shelf.covers[port.id]}<img src={shelf.covers[port.id]} alt="" />{/if}</span>
  <span class="info">
    <span class="name">{shelf.displayName(port)}</span>
    <span class="meta">
      {#if meta}{meta} · {/if}{shelf.kindLabel(port.kind)}{#if shelf.isInstalled(port.id)} · <span class="ok">{tr("search.installed")}</span>{/if}
    </span>
    {@render children?.()}
  </span>
</span>
