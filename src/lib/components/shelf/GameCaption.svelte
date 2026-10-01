<script lang="ts">
  // Under the focused game: its name (renamable in place), what kind of port it is, whether
  // the game file is there, and its actions. The main action is the same as confirming.
  import { openUrl } from "@tauri-apps/plugin-opener";
  import SystemLogo from "$lib/components/media/SystemLogo.svelte";
  import { shelf } from "$lib/shelf.svelte";
  import { tr } from "$lib/prefs.svelte";
  import PlaytimeBadge from "./PlaytimeBadge.svelte";
  import TrophyIcon from "$lib/components/ui/TrophyIcon.svelte";
  import type { Console, Port } from "$lib/types";

  let {
    port,
    systemId,
    system,
    onconfirm,
    onsettings,
    onachievements,
    onvariant,
    showAll,
  }: {
    port: Port; systemId: string; system: Console; showAll: boolean;
    onconfirm: () => void; onsettings: () => void; onachievements: () => void; onvariant: (port: Port) => void;
  } = $props();
  /** Other ports of the same game that can be shown instead (installed ones only, unless every port is). */
  let variants = $derived(shelf.variantsOf(port).filter((p) => showAll || shelf.isInstalled(p.id)));
  let achievements = $derived(shelf.achievements[port.id]);

  let installed = $derived(shelf.isInstalled(port.id));
  let ready = $derived(shelf.romReady(port.id));
  let progress = $derived(shelf.installing[port.id]);
  let disc = $derived(shelf.isDisc(port));
  let played = $derived(shelf.playtime[port.id]);

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
      <span class="name">{shelf.displayName(port)}</span>
      <button class="icon" onclick={startRename} title={tr("game.renameHint")} aria-label={tr("game.rename")}>✎</button>
    </h2>
  {/if}
  <p class="meta">
    {#if variants.length > 1}
      <span class="field variant">
        <select value={port.id} aria-label={tr("game.version")} title={tr("game.versionHint")}
          onchange={(e) => { const next = variants.find((p) => p.id === e.currentTarget.value); if (next) onvariant(next); e.currentTarget.blur(); }}>
          {#each variants as v (v.id)}
            <option value={v.id}>{shelf.variantLabel(v)}{shelf.isInstalled(v.id) ? " ✓" : ""}</option>
          {/each}
        </select>
      </span> ·
    {/if}
    {#if shelf.library?.overrides?.[port.id]?.name}{port.name} · {/if}{shelf.kindLabel(port.kind)}{#if shelf.authorsOf(port)} · {tr("game.by", { authors: shelf.authorsOf(port) })}{/if}{installed ? "" : ` · ${tr("game.notInstalled")}`}
  </p>
  <!-- One slot of fixed height for what changes from game to game, so the shelf above never moves. -->
  <div class="slot">
    {#if shelf.selfManaged(port.id)}
      <p class="rom">{tr("game.asksForFile")}</p>
    {:else if (installed && shelf.roms[port.id] && !ready) || (port.rom && !(installed && ready))}
      {#if installed && shelf.roms[port.id] && !ready}
        <p class="rom missing">
          {#if shelf.roms[port.id].wrong}{tr("game.wrongVersion", { found: shelf.roms[port.id].wrong! })}{:else}{tr(disc ? "game.missingDisc" : "game.missingFile")}{/if}
        </p>
      {/if}
      {#if port.rom && !(installed && ready)}
        <p class="rom needs">{tr("game.needs", { release: port.rom.needs })}</p>
      {/if}
    {:else if played}
      <PlaytimeBadge {played} />
    {/if}
  </div>

  <div class="actions">
    {#if installed && ready}
      <button class="primary" onclick={onconfirm}>{shelf.selfManaged(port.id) ? tr("game.start") : tr("game.play")}</button>
      <button class="ghost" onclick={() => shelf.selectRom(port)}>{tr(disc ? "game.changeDisc" : "game.changeFile")}</button>
      <button class="ghost" onclick={onsettings}>{tr("game.settings")}</button>
    {:else if installed}
      <button class="primary" onclick={onconfirm}>{tr(disc ? "game.selectDisc" : "game.selectFile")}</button>
      <button class="ghost" onclick={onsettings}>{tr("game.settings")}</button>
    {:else if progress}
      <button class="primary progress" disabled style="--p:{progress.total ? (progress.done / progress.total) * 100 : 0}%">{shelf.installLabel(port.id)}</button>
    {:else if shelf.unavailable(port)}
      <button class="primary" disabled>{shelf.platformLabel(port)}</button>
    {:else}
      <button class="primary" onclick={onconfirm}>{shelf.canInstall(port) ? tr("game.install") : tr("game.getIt")}</button>
    {/if}
    {#if achievements}
      <button class="ghost ach" onclick={onachievements}><TrophyIcon size={18} /> {tr("ach.button", { n: achievements.unlocked, total: achievements.total })}</button>
    {/if}
    {#if port.available !== false}<button class="ghost" onclick={() => openUrl(port.repo)}>{tr("game.projectPage")}</button>{/if}
  </div>
  <div class="actions small">
    <button class="link" onclick={() => shelf.pickCover(port)}>{tr("game.chooseCover")}</button>
    <button class="link" onclick={() => shelf.findCover(port)}>{tr("game.findCover")}</button>
  </div>
  <p class="status">{shelf.status ?? ""}</p>
</div>

<style>
  /* Every row has a fixed height: the caption is the same size for every game. */
  .caption { text-align: center; display: flex; flex-direction: column; align-items: center; }
  h2 { margin: 0; height: 36px; max-width: 100%; font-size: 28px; display: inline-flex; align-items: center; gap: 8px; }
  .name, .meta, .status { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  p { margin: 4px 0; color: var(--muted); font-size: 17px; }
  .system { height: 22px; margin-bottom: 6px; }
  .meta { max-width: 100%; height: 30px; display: flex; align-items: center; justify-content: center; gap: 6px; }
  .variant select { height: 28px; font-size: 14px; }
  .slot { height: 68px; width: 100%; display: flex; flex-direction: column; justify-content: center; overflow: hidden; }
  .slot p { margin: 2px 0; }
  .slot :global(.badges) { margin: 0; }
  .icon { background: none; border: 0; color: var(--muted); font-size: 18px; padding: 2px 4px; border-radius: 4px; }
  .icon:hover, .icon:focus-visible { color: var(--accent); }
  .rename {
    height: 36px; box-sizing: border-box;
    font: 600 26px/1.2 var(--font-text); text-align: center; color: inherit;
    background: var(--surface); border: 1px solid var(--accent); border-radius: 6px; padding: 2px 10px; width: min(520px, 90%);
  }
  .rom { font-size: 15px; }
  .rom.missing { color: var(--accent); }
  .rom.needs { font-size: 14px; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .actions { display: flex; gap: 10px; justify-content: center; margin-top: 10px; height: 44px; align-items: center; }
  .actions.small { margin-top: 6px; gap: 16px; height: 24px; }
  .ach { display: inline-flex; align-items: center; gap: 6px; }
  .primary.progress {
    color: var(--on-accent); opacity: 1; cursor: progress;
    background: linear-gradient(90deg, var(--accent) var(--p), color-mix(in srgb, var(--accent) 45%, var(--surface)) var(--p));
  }
  .status { color: var(--ok); height: 22px; max-width: 100%; }
</style>
