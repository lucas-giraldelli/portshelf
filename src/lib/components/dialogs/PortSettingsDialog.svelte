<script lang="ts">
  // A port's own settings files, one tab per file, edited in place.
  import { invoke } from "@tauri-apps/api/core";
  import Modal from "$lib/components/ui/Modal.svelte";
  import { tr } from "$lib/prefs.svelte";
  import { shelf } from "$lib/shelf.svelte";
  import type { Port } from "$lib/types";

  let { port, onclose }: { port: Port; onclose: () => void } = $props();

  let config = $state<Record<string, Record<string, unknown>> | null>(null);
  let tab = $state("");
  let tabs = $derived(Object.keys(config ?? {}));

  $effect(() => {
    invoke<Record<string, Record<string, unknown>>>("get_config", { id: port.id })
      .then((c) => {
        config = c;
        tab = Object.keys(c)[0] ?? "";
      })
      .catch((e) => shelf.fail(e));
  });

  async function set(file: string, key: string, value: unknown) {
    if (!config) return;
    try {
      await invoke("set_config", { id: port.id, file, key, value });
      config[file][key] = value;
    } catch (e) {
      shelf.fail(e);
    }
  }

  export function handlePad(action: string) {
    const i = tabs.indexOf(tab);
    if (action === "left" || action === "lb") tab = tabs[Math.max(0, i - 1)] ?? tab;
    else if (action === "right" || action === "rb") tab = tabs[Math.min(tabs.length - 1, i + 1)] ?? tab;
    else if (action === "y") onclose();
    else return false;
    return true;
  }

  const pretty = (k: string) => k.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
</script>

<Modal title={port.name} label={tr("settings.aria", { name: shelf.displayName(port) })} {onclose}>
  {#if config && tabs.length}
    <div class="tabs" role="tablist">
      {#each tabs as t (t)}
        <button role="tab" aria-selected={tab === t} class:on={tab === t} onclick={() => (tab = t)}>{pretty(t)}</button>
      {/each}
    </div>
    <div class="options">
      {#each Object.entries(config[tab] ?? {}) as [key, value] (key)}
        <label>
          <span>{pretty(key)}</span>
          {#if typeof value === "boolean"}
            <input type="checkbox" checked={value} onchange={(e) => set(tab, key, e.currentTarget.checked)} />
          {:else if typeof value === "number"}
            <input type="number" {value} onchange={(e) => set(tab, key, Number(e.currentTarget.value))} />
          {:else if typeof value === "string"}
            <input type="text" {value} onchange={(e) => set(tab, key, e.currentTarget.value)} />
          {:else}
            <code>{JSON.stringify(value).slice(0, 40)}</code>
          {/if}
        </label>
      {/each}
    </div>
  {:else if config}
    <p class="muted">{tr("settings.none")}</p>
  {/if}
</Modal>

<style>
  .tabs { display: flex; gap: 4px; flex-wrap: wrap; margin-bottom: 8px; }
  .tabs button { background: var(--surface); border: 0; border-radius: 6px; padding: 4px 10px; }
  .tabs button.on { background: var(--accent); color: var(--on-accent); }
  .options { display: grid; gap: 6px; }
  label { display: flex; justify-content: space-between; align-items: center; gap: 12px; padding: 4px 0; border-bottom: 1px solid var(--surface); }
  input[type="text"], input[type="number"] {
    width: 150px; background: var(--bg); color: inherit; border: 1px solid var(--border); border-radius: 4px; padding: 3px 6px;
  }
</style>
