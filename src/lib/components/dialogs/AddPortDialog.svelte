<script lang="ts">
  // After picking a port's program: confirm which catalog port it is (guessed from the file and
  // folder names), pick another, or describe a new one.
  import Dialog from "$lib/components/ui/Dialog.svelte";
  import { tr } from "$lib/prefs.svelte";
  import { guessPort, shelf } from "$lib/shelf.svelte";

  let { exec, consoleId, onclose, onadded }: { exec: string; consoleId: string; onclose: () => void; onadded: (id: string) => void } = $props();

  // svelte-ignore state_referenced_locally
  let choice = $state(guessPort(exec));
  let name = $state("");
  // svelte-ignore state_referenced_locally
  let system = $state(consoleId);

  async function add() {
    const id = await shelf.addInstall(exec, choice, name, system);
    if (id) onadded(id);
  }
</script>

<Dialog {onclose} labelledby="add-title" width={560}>
  <div class="body">
    <h2 id="add-title">{tr("add.title")}</h2>
    <p class="muted file">{exec}</p>
    <label class="field">
      <span>{tr("add.thisIs")}</span>
      <select bind:value={choice}>
        {#each shelf.consoles as [id, info] (id)}
          <optgroup label={info.name}>
            {#each shelf.catalog?.ports.filter((p) => p.console === id) ?? [] as port (port.id)}
              <option value={port.id}>{shelf.displayName(port)}{shelf.isInstalled(port.id) ? ` ${tr("add.installed")}` : ""}</option>
            {/each}
          </optgroup>
        {/each}
        <option value="new">{tr("add.new")}</option>
      </select>
    </label>
    {#if choice === "new"}
      <label class="field">
        <span>{tr("add.name")}</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input type="text" bind:value={name} placeholder={tr("game.nameField")} autofocus />
      </label>
      <label class="field">
        <span>{tr("add.system")}</span>
        <select bind:value={system}>
          {#each shelf.consoles as [id, info] (id)}
            <option value={id}>{info.name}</option>
          {/each}
        </select>
      </label>
    {:else if shelf.isInstalled(choice)}
      <p class="warn">{tr("add.replaces")}</p>
    {/if}
    <div class="choices">
      <button class="tool" onclick={onclose}>{tr("common.cancel")}</button>
      <button class="tool accent" onclick={add} disabled={choice === "new" && !name.trim()}>{tr("common.add")}</button>
    </div>
  </div>
</Dialog>

<style>
  .body { padding: 22px; }
  h2 { margin: 0 0 12px; font-size: 20px; text-align: center; }
  .file { font-size: 13px; word-break: break-all; margin: 0 0 14px; }
  .warn { font-size: 14px; margin: 0 0 12px; }
  .choices { display: flex; gap: 12px; justify-content: center; margin-top: 6px; }
  .choices .tool { min-width: 110px; }
</style>
