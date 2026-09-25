<script lang="ts">
  // Every port the catalog knows for one system, with links to the projects.
  import { openUrl } from "@tauri-apps/plugin-opener";
  import Modal from "$lib/components/ui/Modal.svelte";
  import PortRow from "$lib/components/ui/PortRow.svelte";
  import { tr } from "$lib/prefs.svelte";
  import { shelf } from "$lib/shelf.svelte";
  import type { Port } from "$lib/types";

  let { consoleId, onclose, onshow }: { consoleId: string; onclose: () => void; onshow: (port: Port) => void } = $props();
  let name = $derived(shelf.catalog?.consoles[consoleId].name ?? "");

  const host = (url: string) => {
    try {
      const u = new URL(url);
      const repo = u.pathname.split("/").slice(1, 3).join("/");
      return ["github.com", "gitlab.com"].includes(u.hostname) ? `${u.hostname}/${repo}` : u.hostname.replace(/^www\./, "");
    } catch {
      return url;
    }
  };
</script>

<Modal title={name} label={tr("known.aria", { system: name })} {onclose} width={760}>
  <p class="muted">{tr("known.intro")}</p>
  <ul>
    {#each shelf.knownPorts(consoleId) as port (port.id)}
      <li>
        <button class="show" onclick={() => onshow(port)} aria-label={tr("known.show", { name: shelf.displayName(port) })}>
          <PortRow {port} />
        </button>
        <button class="link" onclick={() => openUrl(port.repo)}>{host(port.repo)} ↗</button>
      </li>
    {/each}
  </ul>
</Modal>

<style>
  ul { list-style: none; padding: 0; margin: 12px 0 0; display: grid; gap: 10px; grid-template-columns: repeat(auto-fill, minmax(270px, 1fr)); }
  li { display: flex; flex-direction: column; align-items: flex-start; gap: 4px; padding: 8px; border-radius: var(--radius); background: var(--surface); }
  .show { background: none; border: 0; padding: 0; text-align: left; }
  .show:hover :global(.name), .show:focus-visible :global(.name) { color: var(--accent); }
  .link { font-size: 14px; margin-left: 72px; max-width: calc(100% - 72px); text-align: left; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
</style>
