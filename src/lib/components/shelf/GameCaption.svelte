<script lang="ts">
  // Under the focused game: its name (renamable in place), what kind of port it is, whether
  // the game file is there, and its actions. The main action is the same as confirming.
  import { openUrl } from "@tauri-apps/plugin-opener";
  import SystemLogo from "$lib/components/media/SystemLogo.svelte";
  import { shelf } from "$lib/shelf.svelte";
  import { tr } from "$lib/prefs.svelte";
  import type { Console, Port } from "$lib/types";

  let {
    port,
    systemId,
    system,
    onconfirm,
    onsettings,
  }: { port: Port; systemId: string; system: Console; onconfirm: () => void; onsettings: () => void } = $props();

  let installed = $derived(shelf.isInstalled(port.id));
  let ready = $derived(shelf.romReady(port.id));
  let progress = $derived(shelf.installing[port.id]);

  // Renaming: the title becomes a text field.
  let editing = $state(false);
  let draft = $state("");
  export function startRename() {
    draft = shelf.displayName(port);
    editing = true;
  }
  export const isEditing = () => editing;
  function save() {
    if (!editing) return;
    editing = false;
    shelf.rename(port, draft);
  }
</script>

<div class="caption">
  <p class="system"><SystemLogo id={systemId} name={system.name} height={22} /></p>
  {#if editing}
    <!-- svelte-ignore a11y_autofocus -->
    <input class="rename" bind:value={draft} autofocus aria-label={tr("game.nameField")} onblur={save}
      onkeydown={(e) => { if (e.key === "Enter") save(); if (e.key === "Escape") { e.stopPropagation(); editing = false; } }} />
  {:else}
    <h2>
      {shelf.displayName(port)}
      <button class="icon" onclick={startRename} title={tr("game.renameHint")} aria-label={tr("game.rename")}>✎</button>
    </h2>
    {#if shelf.library?.overrides?.[port.id]?.name}<p class="original">{port.name}</p>{/if}
  {/if}
  <p>
    {shelf.kindLabel(port.kind)}{#if shelf.authorsOf(port)} · {tr("game.by", { authors: shelf.authorsOf(port) })}{/if}{installed ? "" : ` · ${tr("game.notInstalled")}`}
  </p>
  {#if shelf.selfManaged(port.id)}
    <p class="rom">{tr("game.asksForFile")}</p>
  {:else if installed && shelf.roms[port.id] && !ready}
    <p class="rom missing">{tr("game.missingFile")}</p>
  {/if}

  <div class="actions">
    {#if installed && ready}
      <button class="primary" onclick={onconfirm}>{shelf.selfManaged(port.id) ? tr("game.start") : tr("game.play")}</button>
      <button class="ghost" onclick={() => shelf.selectRom(port)}>{tr("game.changeFile")}</button>
      <button class="ghost" onclick={onsettings}>{tr("game.settings")}</button>
    {:else if installed}
      <button class="primary" onclick={onconfirm}>{tr("game.selectFile")}</button>
      <button class="ghost" onclick={onsettings}>{tr("game.settings")}</button>
    {:else if progress}
      <button class="primary progress" disabled style="--p:{progress.total ? (progress.done / progress.total) * 100 : 0}%">{shelf.installLabel(port.id)}</button>
    {:else if shelf.unavailable(port)}
      <button class="primary" disabled>{shelf.platformLabel(port)}</button>
    {:else}
      <button class="primary" onclick={onconfirm}>{shelf.canInstall(port) ? tr("game.install") : tr("game.getIt")}</button>
    {/if}
    <button class="ghost" onclick={() => openUrl(port.repo)}>{tr("game.projectPage")}</button>
  </div>
  <div class="actions small">
    <button class="link" onclick={() => shelf.pickCover(port)}>{tr("game.chooseCover")}</button>
    <button class="link" onclick={() => shelf.findCover(port)}>{tr("game.findCover")}</button>
  </div>
  {#if shelf.status}<p class="status">{shelf.status}</p>{/if}
</div>

<style>
  .caption { text-align: center; min-height: 170px; }
  h2 { margin: 0; font-size: 28px; display: inline-flex; align-items: center; gap: 8px; }
  p { margin: 4px 0; color: var(--muted); font-size: 17px; }
  .system { margin-bottom: 6px; }
  .original { font-size: 14px; }
  .icon { background: none; border: 0; color: var(--muted); font-size: 18px; padding: 2px 4px; border-radius: 4px; }
  .icon:hover, .icon:focus-visible { color: var(--accent); }
  .rename {
    font: 600 26px/1.2 var(--font-text); text-align: center; color: inherit;
    background: var(--surface); border: 1px solid var(--accent); border-radius: 6px; padding: 2px 10px; width: min(520px, 90%);
  }
  .rom { font-size: 15px; }
  .rom.missing { color: var(--accent); }
  .actions { display: flex; gap: 10px; justify-content: center; margin-top: 14px; flex-wrap: wrap; }
  .actions.small { margin-top: 6px; gap: 16px; }
  .primary.progress {
    color: var(--on-accent); opacity: 1; cursor: progress;
    background: linear-gradient(90deg, var(--accent) var(--p), color-mix(in srgb, var(--accent) 45%, var(--surface)) var(--p));
  }
  .status { color: var(--ok); }
</style>
