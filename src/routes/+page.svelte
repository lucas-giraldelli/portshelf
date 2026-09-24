<script lang="ts">
  // The shelf: pick a system, then a game on it, like RetroArch. This page only keeps track of
  // what is in view and which dialog is open, and routes the keyboard and the controller.
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { open } from "@tauri-apps/plugin-dialog";
  import { fade, fly, scale } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import Header from "$lib/components/shelf/Header.svelte";
  import Carousel from "$lib/components/shelf/Carousel.svelte";
  import SystemCaption from "$lib/components/shelf/SystemCaption.svelte";
  import GameCaption from "$lib/components/shelf/GameCaption.svelte";
  import ControlHints from "$lib/components/shelf/ControlHints.svelte";
  import Cartridge from "$lib/components/media/Cartridge.svelte";
  import Disc from "$lib/components/media/Disc.svelte";
  import ConsoleIcon from "$lib/components/media/ConsoleIcon.svelte";
  import QuitDialog from "$lib/components/dialogs/QuitDialog.svelte";
  import SettingsDialog from "$lib/components/dialogs/SettingsDialog.svelte";
  import AddPortDialog from "$lib/components/dialogs/AddPortDialog.svelte";
  import SearchDialog from "$lib/components/dialogs/SearchDialog.svelte";
  import KnownPortsPanel from "$lib/components/panels/KnownPortsPanel.svelte";
  import PortSettingsPanel from "$lib/components/panels/PortSettingsPanel.svelte";
  import { nextTheme, prefs, tr } from "$lib/prefs.svelte";
  import { shelf } from "$lib/shelf.svelte";
  import type { Port } from "$lib/types";

  // ---- what is in view ----

  let view = $state<"systems" | "games">("systems");
  let systemIndex = $state(0);
  let gameIndex = $state(0);
  let showAll = $state(false);

  let systems = $derived.by(() => {
    const catalog = shelf.catalog;
    if (!catalog) return [];
    return Object.entries(catalog.consoles)
      .map(([id, info]) => {
        const ports = catalog.ports
          .filter((p) => p.console === id && (showAll || shelf.isInstalled(p.id)))
          .sort((a, b) => Number(shelf.isInstalled(b.id)) - Number(shelf.isInstalled(a.id)) || a.name.localeCompare(b.name));
        return { id, info, ports, installed: ports.filter((p) => shelf.isInstalled(p.id)).length };
      })
      .filter((s) => s.ports.length > 0)
      .sort((a, b) => a.info.year - b.info.year || a.info.name.localeCompare(b.info.name));
  });
  let system = $derived(systems[systemIndex]);
  let game = $derived(system?.ports[gameIndex]);

  $effect(() => {
    if (view === "games" && game) shelf.refreshRom(game);
  });

  /** What confirming does right now, for the footer. */
  let primaryLabel = $derived(
    view === "systems" ? tr("game.open")
    : !game ? ""
    : shelf.isInstalled(game.id) ? (shelf.romReady(game.id) ? (shelf.selfManaged(game.id) ? tr("game.start") : tr("game.play")) : tr("game.selectFile"))
    : shelf.unavailable(game) ? tr("game.projectPage")
    : shelf.canInstall(game) ? tr("game.install") : tr("game.getIt"));

  const clamp = (i: number, n: number) => Math.max(0, Math.min(n - 1, i));
  const wrap = (i: number, n: number) => (n ? ((i % n) + n) % n : 0);

  // ---- dialogs and panels (one at a time) ----

  type Overlay =
    | { kind: "quit" }
    | { kind: "settings" }
    | { kind: "add"; exec: string }
    | { kind: "search" }
    | { kind: "known"; console: string }
    | { kind: "port"; port: Port };
  let overlay = $state<Overlay | null>(null);
  /** The open dialog's own key and controller handling, when it has any. */
  let overlayRef = $state<{ handleKey?: (e: KeyboardEvent) => boolean; handlePad?: (action: string) => boolean }>();
  let caption = $state<GameCaption>();

  const close = () => (overlay = null);
  const openOverlay = (next: Overlay) => {
    if (!overlay && !caption?.isEditing()) overlay = next;
  };

  async function startAddPort() {
    overlay = null;
    const exec = await open({ title: tr("pick.program") });
    if (typeof exec === "string") overlay = { kind: "add", exec };
  }
  function portAdded(id: string) {
    overlay = null;
    const port = shelf.portById(id);
    if (port) showOnShelf(port);
  }
  function openPortSettings() {
    if (view === "games" && game && shelf.isInstalled(game.id)) openOverlay({ kind: "port", port: game });
  }

  /** Jump to a port on its shelf (showing every port if it is not installed). */
  function showOnShelf(port: Port) {
    if (!shelf.isInstalled(port.id)) showAll = true;
    overlay = null;
    systemIndex = Math.max(0, systems.findIndex((s) => s.id === port.console));
    gameIndex = Math.max(0, systems[systemIndex]?.ports.findIndex((p) => p.id === port.id) ?? 0);
    view = "games";
  }

  // ---- navigation ----

  function move(delta: number) {
    if (overlay) return;
    if (view === "systems") systemIndex = wrap(systemIndex + delta, systems.length);
    else if (system) gameIndex = clamp(gameIndex + delta, system.ports.length);
  }
  function switchSystem(delta: number) {
    if (view !== "games") return;
    systemIndex = wrap(systemIndex + delta, systems.length);
    gameIndex = 0;
  }
  function toggleAll() {
    showAll = !showAll;
    systemIndex = clamp(systemIndex, systems.length);
    gameIndex = 0;
  }

  async function confirm() {
    if (overlay) return;
    if (view === "systems") {
      if (!system) return;
      view = "games";
      gameIndex = 0;
      return;
    }
    if (!game) return;
    const port = game;
    if (shelf.isInstalled(port.id)) {
      if (shelf.romReady(port.id)) await shelf.play(port);
      else await shelf.selectRom(port);
    } else if (!shelf.unavailable(port) && shelf.canInstall(port)) {
      // Installed ports sort first; keep the focus on the one just installed.
      if (await shelf.installPort(port)) gameIndex = Math.max(0, system?.ports.findIndex((p) => p.id === port.id) ?? 0);
    } else openUrl(port.repo);
  }

  /** Back: close what is open, leave the games, or ask whether to quit. */
  function back() {
    if (overlay) overlay = null;
    else if (view === "games") view = "systems";
    else overlay = { kind: "quit" };
  }
  function answerQuit(yes: boolean) {
    overlay = null;
    if (yes) invoke("quit");
  }

  // ---- input ----

  // The footer shows the controls of whatever was used last; with the controller the mouse
  // pointer hides until the mouse moves again.
  let inputMode = $state<"keys" | "pad">("keys");
  $effect(() => {
    const pad = inputMode === "pad";
    document.body.classList.toggle("pad-mode", pad);
    invoke("set_cursor_visible", { visible: !pad }).catch(() => {});
  });

  function onKey(e: KeyboardEvent) {
    inputMode = "keys";
    const inField = e.target instanceof HTMLInputElement;
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
      e.preventDefault();
      openOverlay({ kind: "search" });
      return;
    }
    if (overlay) {
      if (overlayRef?.handleKey?.(e)) e.preventDefault();
      else if (e.key === "Escape" || (e.key === "Backspace" && !inField)) {
        e.preventDefault();
        close();
      }
      return;
    }
    if (inField) return;
    const actions: Record<string, () => void> = {
      ArrowLeft: () => move(-1), ArrowRight: () => move(1), a: () => move(-1), d: () => move(1),
      Enter: confirm, " ": confirm, Escape: back, Backspace: back, Tab: toggleAll,
      s: openPortSettings, r: () => view === "games" && caption?.startRename(),
      i: () => shelf.chooseSystemFile(system?.id), p: startAddPort,
      o: () => openOverlay({ kind: "settings" }), k: () => system && openOverlay({ kind: "known", console: system.id }),
      t: () => shelf.flash(tr("msg.theme", { name: nextTheme() })),
      m: () => (prefs.mode = prefs.mode === "dark" ? "light" : "dark"),
      l: () => (prefs.lang = prefs.lang === "en" ? "pt-BR" : "en"),
    };
    const action = actions[e.key];
    if (action) {
      e.preventDefault();
      action();
    }
  }

  // Controller, read by the backend and forwarded as "pad" events: D-pad / stick browse,
  // A confirms, B goes back, Y settings, X known ports, Select every known port, Start search,
  // LB / RB switch system while looking at games.
  function onPad(action: string) {
    inputMode = "pad";
    if (overlay) {
      if (!overlayRef?.handlePad?.(action) && action === "b") close();
      return;
    }
    const actions: Record<string, () => void> = {
      left: () => move(-1), right: () => move(1), a: confirm, b: back,
      y: () => (view === "systems" ? openOverlay({ kind: "settings" }) : openPortSettings()),
      x: () => system && openOverlay({ kind: "known", console: system.id }),
      select: toggleAll, start: () => openOverlay({ kind: "search" }),
      lb: () => switchSystem(-1), rb: () => switchSystem(1),
    };
    actions[action]?.();
  }
  listen<string>("pad", (e) => onPad(e.payload));

  // New ports and game files are picked up whenever the window comes back into focus.
  getCurrentWindow().onFocusChanged(({ payload: focused }) => {
    if (focused) shelf.refreshAll();
  });

  shelf.load();
</script>

<svelte:window onkeydown={onKey} onmousedown={() => (inputMode = "keys")} onmousemove={(e) => { if (e.movementX || e.movementY) inputMode = "keys"; }} />

<main style="--system: {system?.info.color ?? 'var(--accent)'}">
  <Header onsettings={() => openOverlay({ kind: "settings" })} />

  {#if shelf.error}
    <p class="error" role="alert">{shelf.error} <button class="ghost" onclick={() => (shelf.error = "")}>{tr("common.dismiss")}</button></p>
  {/if}

  <!-- Both views sit on top of each other so the enter and leave transitions overlap. -->
  <div class="stage">
    {#if view === "systems"}
      <div class="view" in:fade={{ duration: 220, delay: 120 }} out:scale={{ start: 1.6, opacity: 0, duration: 260, easing: cubicOut }}>
        <Carousel items={systems} key={(s) => s.id} focused={systemIndex} spacing={520} loop label={tr("systems.aria")}
          onmove={move} onfocus={(i) => (systemIndex = i)} onactivate={confirm}>
          {#snippet item(s)}
            <ConsoleIcon id={s.id} size={460} />
          {/snippet}
        </Carousel>
        {#if system}
          <SystemCaption id={system.id} info={system.info} installed={system.installed} onknown={() => openOverlay({ kind: "known", console: system.id })} />
        {/if}
      </div>
    {:else if system}
      <div class="view" out:fade={{ duration: 160 }}>
        <Carousel items={system.ports} key={(p) => p.id} focused={gameIndex} spacing={440} label={system.info.name}
          onmove={move} onfocus={(i) => (gameIndex = i)} onactivate={confirm}>
          {#snippet item(port, i)}
            <div in:fly={{ y: 220, duration: 420, delay: 200 + Math.abs(i - gameIndex) * 70, easing: cubicOut }}>
              {#if system.info.media === "disc"}
                <Disc name={shelf.displayName(port)} cover={shelf.covers[port.id]} console={port.console} installed={shelf.isInstalled(port.id)} size={2.2} />
              {:else}
                <Cartridge name={shelf.displayName(port)} cover={shelf.covers[port.id]} console={port.console} shell={port.shell} installed={shelf.isInstalled(port.id)} size={2.2} />
              {/if}
            </div>
          {/snippet}
        </Carousel>
        {#if game}
          <GameCaption bind:this={caption} port={game} systemId={system.id} system={system.info} onconfirm={confirm} onsettings={openPortSettings} />
        {/if}
      </div>
    {/if}
  </div>

  <ControlHints {view} pad={inputMode === "pad"} primary={primaryLabel} {showAll} />
</main>

{#if overlay?.kind === "quit"}
  <QuitDialog bind:this={overlayRef} pad={inputMode === "pad"} onanswer={answerQuit} />
{:else if overlay?.kind === "settings" && shelf.catalog}
  <SettingsDialog onclose={close} onaddport={startAddPort} />
{:else if overlay?.kind === "add" && shelf.catalog}
  <AddPortDialog exec={overlay.exec} consoleId={system?.id ?? "n64"} onclose={close} onadded={portAdded} />
{:else if overlay?.kind === "search" && shelf.catalog}
  <SearchDialog bind:this={overlayRef} onclose={close} onpick={showOnShelf} />
{:else if overlay?.kind === "known" && shelf.catalog}
  <KnownPortsPanel consoleId={overlay.console} onclose={close} onshow={showOnShelf} />
{:else if overlay?.kind === "port"}
  <PortSettingsPanel bind:this={overlayRef} port={overlay.port} onclose={close} />
{/if}

<style>
  main {
    height: 100vh; display: flex; flex-direction: column; box-sizing: border-box; padding: 16px 28px;
    background: radial-gradient(ellipse at 50% 55%, color-mix(in srgb, var(--system) 30%, transparent), transparent 65%), var(--bg);
    transition: background 0.4s ease;
  }
  .error { background: var(--danger); padding: 8px 12px; border-radius: var(--radius); }
  .stage { position: relative; flex: 1; }
  .view { position: absolute; inset: 0; display: flex; flex-direction: column; }
</style>
