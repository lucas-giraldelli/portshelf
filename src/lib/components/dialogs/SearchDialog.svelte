<script lang="ts">
  // Ctrl+F / Start: find a game in every system and jump to it on its shelf.
  import Dialog from "$lib/components/ui/Dialog.svelte";
  import PortRow from "$lib/components/ui/PortRow.svelte";
  import { tr } from "$lib/prefs.svelte";
  import { shelf } from "$lib/shelf.svelte";
  import type { Port } from "$lib/types";

  let { onclose, onpick }: { onclose: () => void; onpick: (port: Port) => void } = $props();

  let query = $state("");
  let index = $state(0);
  const fold = (t: string) => t.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
  let results = $derived.by(() => {
    const words = fold(query).split(/\s+/).filter(Boolean);
    return (shelf.catalog?.ports ?? [])
      .filter((p) => {
        const haystack = fold([shelf.displayName(p), p.name, p.title ?? "", shelf.catalog!.consoles[p.console].name].join(" "));
        return words.every((w) => haystack.includes(w));
      })
      .sort((a, b) => Number(shelf.isInstalled(b.id)) - Number(shelf.isInstalled(a.id)) || shelf.displayName(a).localeCompare(shelf.displayName(b)));
  });

  const step = (delta: number) => (index = Math.max(0, Math.min(results.length - 1, index + delta)));

  export function handleKey(e: KeyboardEvent) {
    if (e.key === "ArrowDown") step(1);
    else if (e.key === "ArrowUp") step(-1);
    else if (e.key === "Enter" && results[index]) onpick(results[index]);
    else return false;
    return true;
  }
  export function handlePad(action: string) {
    if (action === "down") step(1);
    else if (action === "up") step(-1);
    else if (action === "a" && results[index]) onpick(results[index]);
    else if (action === "start") onclose();
    else return false;
    return true;
  }
</script>

<Dialog {onclose} label={tr("search.aria")} width={640} align="top">
  <!-- svelte-ignore a11y_autofocus -->
  <input
    type="search" placeholder={tr("search.placeholder")} autofocus bind:value={query} oninput={() => (index = 0)}
    aria-controls="search-results" aria-activedescendant={results[index] ? `result-${results[index].id}` : undefined}
  />
  <ul id="search-results" role="listbox">
    {#each results as port, i (port.id)}
      <li id="result-{port.id}" role="option" aria-selected={i === index}>
        <button class:on={i === index} onclick={() => onpick(port)} onmouseenter={() => (index = i)}>
          <PortRow {port} meta={shelf.catalog?.consoles[port.console].name} />
        </button>
      </li>
    {:else}
      <li class="none">{tr("search.none", { query })}</li>
    {/each}
  </ul>
</Dialog>

<style>
  input {
    width: 100%; box-sizing: border-box; border: 0; border-bottom: 1px solid var(--border); background: transparent;
    color: inherit; font: 18px var(--font-text); padding: 16px 18px; outline: none;
  }
  ul { list-style: none; margin: 0; padding: 6px; max-height: 56vh; overflow-y: auto; }
  button { width: 100%; background: none; border: 0; border-radius: var(--radius); padding: 6px 8px; text-align: left; }
  button.on { background: var(--surface-2); }
  .none { padding: 14px; color: var(--muted); }
</style>
