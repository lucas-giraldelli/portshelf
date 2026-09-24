<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import Cartridge from "$lib/Cartridge.svelte";
  import Disc from "$lib/Disc.svelte";
  import ConsoleIcon from "$lib/ConsoleIcon.svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { fade, fly, scale } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import type { Catalog, Library, Port, RomStatus } from "$lib/types";

  let catalog = $state<Catalog | null>(null);
  let library = $state<Library | null>(null);
  let covers = $state<Record<string, string | null>>({});
  let showAll = $state(false);
  let error = $state("");
  let status = $state("");

  // Two levels, like RetroArch: pick a system, then pick a game on it.
  let view = $state<"systems" | "games">("systems");
  let systemIndex = $state(0);
  let gameIndex = $state(0);
  let settingsOpen = $state(false);
  let config = $state<Record<string, Record<string, unknown>> | null>(null);
  let configTab = $state("");

  const isInstalled = (id: string) => !!library?.installed[id];

  // Whether each installed port has its game file set up; the shelf only starts ready ports.
  let roms = $state<Record<string, RomStatus>>({});
  async function refreshRom(port: Port) {
    if (!isInstalled(port.id)) return;
    try {
      roms[port.id] = await invoke<RomStatus>("rom_status", { id: port.id, console: port.console });
    } catch (e) {
      error = String(e);
    }
  }
  $effect(() => {
    if (view === "games" && game) refreshRom(game);
  });
  const romReady = (id: string) => roms[id]?.ready ?? false;
  const fileName = (path: string) => path.split("/").pop();

  async function selectRom(port: Port) {
    const extensions = catalog?.consoles[port.console].media === "disc" ? ["iso", "rvz", "gcm", "ciso", "wbfs", "nkit.iso"] : ["z64", "n64", "v64"];
    const picked = await open({
      title: `Select the ${catalog?.consoles[port.console].name} game file for ${port.name}`,
      defaultPath: roms[port.id]?.browse_dir,
      filters: [{ name: "Game files", extensions }],
    });
    if (typeof picked !== "string") return;
    try {
      await invoke("select_rom", { id: port.id, path: picked });
      await refreshRom(port);
      status = "Game file ready";
      setTimeout(() => (status = ""), 3000);
    } catch (e) {
      error = String(e);
    }
  }

  let systems = $derived.by(() => {
    if (!catalog) return [];
    return Object.entries(catalog.consoles)
      .map(([id, info]) => {
        const ports = catalog!.ports
          .filter((p) => p.console === id && (showAll || isInstalled(p.id)))
          .sort((a, b) => Number(isInstalled(b.id)) - Number(isInstalled(a.id)) || a.name.localeCompare(b.name));
        return { id, info, ports, installed: ports.filter((p) => isInstalled(p.id)).length };
      })
      .filter((s) => s.ports.length > 0)
      .sort((a, b) => a.info.year - b.info.year || a.info.name.localeCompare(b.info.name));
  });
  let system = $derived(systems[systemIndex]);
  let game = $derived(system?.ports[gameIndex]);

  async function load() {
    try {
      catalog = await invoke<Catalog>("get_catalog");
      library = await invoke<Library>("get_library");
      for (const port of catalog.ports) {
        invoke<string | null>("get_cover", { id: port.id }).then((c) => (covers[port.id] = c));
      }
    } catch (e) {
      error = String(e);
    }
  }

  function move(delta: number) {
    if (settingsOpen) return;
    if (view === "systems") systemIndex = clamp(systemIndex + delta, systems.length);
    else if (system) gameIndex = clamp(gameIndex + delta, system.ports.length);
  }
  const clamp = (i: number, n: number) => Math.max(0, Math.min(n - 1, i));

  async function confirm() {
    if (settingsOpen) return;
    if (view === "systems") {
      if (!system) return;
      view = "games";
      gameIndex = 0;
    } else if (game) {
      if (!isInstalled(game.id)) openUrl(game.repo);
      else if (romReady(game.id)) await play(game);
      else await selectRom(game);
    }
  }

  function toggleAll() {
    if (settingsOpen) return;
    showAll = !showAll;
    systemIndex = clamp(systemIndex, systems.length);
    gameIndex = 0;
  }

  function back() {
    if (settingsOpen) settingsOpen = false;
    else if (view === "games") view = "systems";
  }

  async function play(port: Port) {
    try {
      await invoke("launch", { id: port.id });
      status = `${port.name} started`;
      setTimeout(() => (status = ""), 3000);
    } catch (e) {
      error = String(e);
    }
  }

  async function openSettings() {
    if (view !== "games" || !game || !isInstalled(game.id)) return;
    try {
      config = await invoke("get_config", { id: game.id });
      configTab = Object.keys(config ?? {})[0] ?? "";
      settingsOpen = true;
    } catch (e) {
      error = String(e);
    }
  }

  async function setOption(file: string, key: string, value: unknown) {
    if (!game || !config) return;
    try {
      await invoke("set_config", { id: game.id, file, key, value });
      config[file][key] = value;
    } catch (e) {
      error = String(e);
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement) return;
    const actions: Record<string, () => void> = {
      ArrowLeft: () => move(-1), ArrowRight: () => move(1), a: () => move(-1), d: () => move(1),
      Enter: confirm, " ": confirm, Escape: back, Backspace: back, s: openSettings, Tab: toggleAll,
    };
    const action = actions[e.key];
    if (action) {
      e.preventDefault();
      action();
    }
  }

  // Gamepad: D-pad / left stick to move, A to confirm, B to go back, Y for settings.
  let lastButtons: boolean[] = [];
  let lastAxis = 0;
  function pollGamepad() {
    const pad = navigator.getGamepads?.().find((p) => p);
    if (pad) {
      const pressed = pad.buttons.map((b) => b.pressed);
      const edge = (i: number) => pressed[i] && !lastButtons[i];
      if (edge(0)) confirm();
      if (edge(1)) back();
      if (edge(3)) openSettings();
      if (edge(8)) toggleAll();
      if (edge(14)) move(-1);
      if (edge(15)) move(1);
      const axis = Math.abs(pad.axes[0]) > 0.6 ? Math.sign(pad.axes[0]) : 0;
      if (axis !== 0 && axis !== lastAxis) move(axis);
      lastAxis = axis;
      lastButtons = pressed;
    }
    requestAnimationFrame(pollGamepad);
  }
  requestAnimationFrame(pollGamepad);

  const prettyKey = (k: string) => k.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
  const kindLabel = { recomp: "Static recompilation", decomp: "Decompilation", build: "Builds from your ROM" };

  // Position of item i relative to the focused one.
  const slot = (i: number, focused: number, spacing: number) => {
    const offset = i - focused;
    const distance = Math.abs(offset);
    return `transform: translate(calc(-50% + ${offset * spacing}px), -50%) scale(${Math.max(0.55, 1 - distance * 0.18)}); opacity: ${Math.max(0, 1 - distance * 0.28)}; z-index: ${100 - distance};`;
  };

  load();
</script>

<svelte:window onkeydown={onKey} />

<main style="--accent: {system?.info.color ?? '#f2b04c'}">
  <header>
    <h1>portshelf</h1>
    <label class="toggle">
      <input type="checkbox" bind:checked={showAll} onchange={() => { systemIndex = 0; gameIndex = 0; }} />
      Show every known port
    </label>
    <button class="ghost" onclick={async () => (library = await invoke("rescan"))}>Rescan</button>
  </header>

  {#if error}
    <p class="error" role="alert">{error} <button class="ghost" onclick={() => (error = "")}>dismiss</button></p>
  {/if}

  <div class="stage">
  {#if view === "systems"}
   <div class="view" in:fade={{ duration: 220, delay: 120 }} out:scale={{ start: 1.6, opacity: 0, duration: 260, easing: cubicOut }}>
    <section class="carousel systems" aria-label="Systems">
      {#each systems as s, i (s.id)}
        <button class="slot" class:focused={i === systemIndex} style={slot(i, systemIndex, 520)} onclick={() => (i === systemIndex ? confirm() : (systemIndex = i))}>
          <ConsoleIcon id={s.id} size={460} />
        </button>
      {/each}
    </section>
    {#if system}
      <div class="caption">
        <h2>{system.info.name}</h2>
        <p>{system.installed} installed{showAll ? ` · ${system.ports.length} known` : ""}</p>
      </div>
    {/if}
   </div>
  {:else if system}
   <div class="view" out:fade={{ duration: 160 }}>
    <section class="carousel games" aria-label={system.info.name}>
      {#each system.ports as port, i (port.id)}
        <button class="slot" class:focused={i === gameIndex} style={slot(i, gameIndex, 440)} onclick={() => (i === gameIndex ? confirm() : (gameIndex = i))}
          in:fly={{ y: 220, duration: 420, delay: 200 + Math.abs(i - gameIndex) * 70, easing: cubicOut }}>
          {#if system.info.media === "disc"}
            <Disc name={port.name} cover={covers[port.id]} console={port.console} installed={isInstalled(port.id)} size={2.2} />
          {:else}
            <Cartridge name={port.name} cover={covers[port.id]} console={port.console} shell={port.shell} installed={isInstalled(port.id)} size={2.2} />
          {/if}
        </button>
      {/each}
    </section>
    {#if game}
      <div class="caption">
        <p class="system-name">{system.info.name}</p>
        <h2>{game.name}</h2>
        <p>{kindLabel[game.kind]}{isInstalled(game.id) ? "" : " · not installed"}</p>
        {#if isInstalled(game.id) && roms[game.id]}
          <p class="rom" class:missing={!romReady(game.id)}>
            {#if romReady(game.id)}{roms[game.id].path ? fileName(roms[game.id].path!) : "Game file ready"}{:else}No game file selected yet{/if}
          </p>
        {/if}
        <div class="actions">
          {#if isInstalled(game.id) && romReady(game.id)}
            <button class="primary" onclick={() => play(game!)}>Play</button>
            <button class="ghost" onclick={() => selectRom(game!)}>Change game file</button>
            <button class="ghost" onclick={openSettings}>Settings</button>
          {:else if isInstalled(game.id)}
            <button class="primary" onclick={() => selectRom(game!)}>Select game file</button>
            <button class="ghost" onclick={openSettings}>Settings</button>
          {:else}
            <button class="primary" onclick={() => openUrl(game!.repo)}>Get it</button>
          {/if}
          <button class="ghost" onclick={() => openUrl(game!.repo)}>Project page</button>
        </div>
        {#if status}<p class="status">{status}</p>{/if}
      </div>
    {/if}
   </div>
  {/if}
  </div>

  <footer>
    {#if view === "games"}<span><kbd>Esc</kbd> / <kbd>B</kbd> Systems</span>{/if}
    <span><kbd>←</kbd><kbd>→</kbd> Browse</span>
    <span><kbd>Enter</kbd> / <kbd>A</kbd> {view === "systems" ? "Open" : game && isInstalled(game.id) && !romReady(game.id) ? "Select game file" : "Play"}</span>
    {#if view === "games"}<span><kbd>S</kbd> / <kbd>Y</kbd> Settings</span>{/if}
    <span><kbd>Tab</kbd> / <kbd>Select</kbd> {showAll ? "Installed only" : "Every known port"}</span>
  </footer>
</main>

{#if settingsOpen && config && game}
  <aside aria-label="{game.name} settings">
    <button class="close ghost" onclick={() => (settingsOpen = false)} aria-label="Close">×</button>
    <h2>{game.name}</h2>
    {#if Object.keys(config).length}
      <div class="tabs" role="tablist">
        {#each Object.keys(config) as tab}
          <button role="tab" aria-selected={configTab === tab} class:on={configTab === tab} onclick={() => (configTab = tab)}>{prettyKey(tab)}</button>
        {/each}
      </div>
      <div class="options">
        {#each Object.entries(config[configTab] ?? {}) as [key, value] (key)}
          <label class="option">
            <span>{prettyKey(key)}</span>
            {#if typeof value === "boolean"}
              <input type="checkbox" checked={value} onchange={(e) => setOption(configTab, key, e.currentTarget.checked)} />
            {:else if typeof value === "number"}
              <input type="number" value={value} onchange={(e) => setOption(configTab, key, Number(e.currentTarget.value))} />
            {:else if typeof value === "string"}
              <input type="text" value={value} onchange={(e) => setOption(configTab, key, e.currentTarget.value)} />
            {:else}
              <code>{JSON.stringify(value).slice(0, 40)}</code>
            {/if}
          </label>
        {/each}
      </div>
    {:else}
      <p class="muted">This port has no settings files yet. Start it once and they will show up here.</p>
    {/if}
  </aside>
{/if}

<style>
  :global(body) {
    margin: 0;
    background: #101014;
    color: #eee8df;
    font: 15px/1.4 system-ui, sans-serif;
    overflow: hidden;
  }
  main {
    height: 100vh; display: flex; flex-direction: column; box-sizing: border-box; padding: 16px 28px;
    background: radial-gradient(ellipse at 50% 55%, color-mix(in srgb, var(--accent) 30%, transparent), transparent 65%), #101014;
    transition: background 0.4s ease;
  }
  header { display: flex; align-items: center; gap: 20px; flex-wrap: wrap; }
  h1 { font-size: 22px; letter-spacing: 0.5px; margin: 4px 0; flex: 1; }
  .toggle { display: flex; gap: 8px; align-items: center; color: #bdb4a7; }
  button { font: inherit; color: inherit; cursor: pointer; }
  .ghost { background: none; border: 1px solid #4a4540; border-radius: 6px; padding: 5px 12px; }
  .primary { background: #f2b04c; color: #1a1510; border: 0; border-radius: 8px; padding: 9px 28px; font-weight: 700; }
  .error { background: #5c1e16; padding: 8px 12px; border-radius: 6px; }

  /* Both views sit on top of each other so the enter and leave transitions overlap. */
  .stage { position: relative; flex: 1; }
  .view { position: absolute; inset: 0; display: flex; flex-direction: column; }
  .carousel { position: relative; flex: 1; min-height: 280px; }
  .slot {
    position: absolute; left: 50%; top: 50%; transform-origin: 50% 50%;
    background: none; border: 0; padding: 0;
    transition: transform 0.28s cubic-bezier(0.2, 0.8, 0.2, 1), opacity 0.28s ease;
  }
  .slot:focus-visible { outline: 2px solid #f2b04c; outline-offset: 8px; border-radius: 12px; }

  /* Focused items come alive: consoles and cartridges bob, discs spin slowly. */
  .slot.focused > :global(*) { animation: bob 2.6s ease-in-out infinite; }
  .slot.focused :global(.disc) { animation: spin 9s linear infinite; }
  .slot.focused :global(.cart) { box-shadow: 0 0 0 3px rgba(242, 176, 76, 0.55), 0 24px 32px rgba(0, 0, 0, 0.6); }
  @keyframes bob { 0%, 100% { transform: translateY(0); } 50% { transform: translateY(-10px); } }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) {
    .slot, .slot.focused > :global(*), .slot.focused :global(.disc) { animation: none; transition: none; }
  }

  .caption { text-align: center; min-height: 170px; }
  .caption h2 { margin: 0; font-size: 28px; }
  .caption p { margin: 4px 0; color: #bdb4a7; }
  .system-name { text-transform: uppercase; letter-spacing: 2px; font-size: 12px; }
  .actions { display: flex; gap: 10px; justify-content: center; margin-top: 14px; flex-wrap: wrap; }
  .status { color: #9fd48b; }
  .rom { font-size: 13px; color: #9fd48b !important; }
  .rom.missing { color: #f2b04c !important; }

  footer { display: flex; gap: 22px; justify-content: center; color: #8f877b; font-size: 13px; padding: 8px 0 4px; flex-wrap: wrap; }
  footer span { display: inline-flex; align-items: center; gap: 4px; }
  kbd {
    display: inline-flex; align-items: center; justify-content: center;
    min-width: 20px; height: 20px; padding: 0 5px; box-sizing: border-box;
    border: 1px solid #4a4540; border-radius: 4px; font: 12px/1 system-ui, sans-serif; color: #d6cec2;
  }

  aside {
    position: fixed; top: 0; right: 0; bottom: 0; width: min(440px, 100vw);
    background: #1b1a1f; border-left: 1px solid #333; padding: 24px; overflow-y: auto; box-sizing: border-box;
    box-shadow: -12px 0 32px rgba(0,0,0,0.5);
  }
  .close { position: absolute; top: 12px; right: 12px; font-size: 18px; }
  aside h2 { margin: 0 0 14px; }
  .tabs { display: flex; gap: 4px; flex-wrap: wrap; margin-bottom: 8px; }
  .tabs button { background: #29282e; border: 0; border-radius: 6px; padding: 4px 10px; }
  .tabs button.on { background: #f2b04c; color: #1a1510; }
  .options { display: grid; gap: 6px; }
  .option { display: flex; justify-content: space-between; align-items: center; gap: 12px; padding: 4px 0; border-bottom: 1px solid #29282e; }
  .option input[type="text"], .option input[type="number"] {
    width: 150px; background: #101014; color: inherit; border: 1px solid #4a4540; border-radius: 4px; padding: 3px 6px;
  }
  code { font-size: 12px; color: #9b948a; }
  .muted { color: #9b948a; }
</style>
