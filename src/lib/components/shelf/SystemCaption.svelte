<script lang="ts">
  // Under the focused system: its logo, year, and how many ports are installed and known.
  import SystemLogo from "$lib/components/media/SystemLogo.svelte";
  import { shelf } from "$lib/shelf.svelte";
  import { tr } from "$lib/prefs.svelte";
  import type { Console } from "$lib/types";

  let { id, info, installed, onknown }: { id: string; info: Console; installed: number; onknown: () => void } = $props();
</script>

<div class="caption">
  <h2><SystemLogo {id} name={info.name} height={52} /></h2>
  <p>
    {info.year} · {tr("systems.installed", { count: installed })} ·
    <button class="link inline" onclick={onknown}>{tr("systems.known", { count: shelf.knownPorts(id).length })}</button>
  </p>
</div>

<style>
  .caption { text-align: center; min-height: 170px; }
  h2 { margin: 0 0 8px; color: var(--text); line-height: 1; }
  p { margin: 4px 0; color: var(--muted); font-size: 17px; }
  .link.inline { font-size: inherit; color: var(--text); }
</style>
