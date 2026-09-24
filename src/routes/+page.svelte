<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import Cartridge from "$lib/Cartridge.svelte";
  import Disc from "$lib/Disc.svelte";
  import ConsoleIcon from "$lib/ConsoleIcon.svelte";
  import PadGlyph from "$lib/PadGlyph.svelte";
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
  // Every port the catalog knows for one system, with links to the projects.
  let knownFor = $state<string | null>(null);
  const panelOpen = () => settingsOpen || knownFor !== null || searchOpen;

  // The footer shows the controls of whatever was used last: controller, or keyboard/mouse.
  let inputMode = $state<"keys" | "pad">("keys");
  $effect(() => {
    invoke("set_cursor_visible", { visible: inputMode !== "pad" }).catch(() => {});
  });

  // Ctrl+F: find a game in every system and jump to it on its shelf.
  let searchOpen = $state(false);
  let query = $state("");
  let searchIndex = $state(0);
  const fold = (t: string) => t.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase();
  let results = $derived.by(() => {
    if (!catalog) return [];
    const words = fold(query).split(/\s+/).filter(Boolean);
    return catalog.ports
      .filter((p) => {
        const haystack = fold([displayName(p), p.name, p.title ?? "", catalog!.consoles[p.console].name].join(" "));
        return words.every((w) => haystack.includes(w));
      })
      .sort((a, b) => Number(isInstalled(b.id)) - Number(isInstalled(a.id)) || displayName(a).localeCompare(displayName(b)));
  });
  function openSearch() {
    if (settingsOpen || knownFor || editingName) return;
    query = "";
    searchIndex = 0;
    searchOpen = true;
  }
  function onSearchKey(e: KeyboardEvent) {
    if (e.key === "ArrowDown") { e.preventDefault(); searchIndex = Math.min(searchIndex + 1, results.length - 1); }
    else if (e.key === "ArrowUp") { e.preventDefault(); searchIndex = Math.max(searchIndex - 1, 0); }
    else if (e.key === "Enter" && results[searchIndex]) { e.preventDefault(); pickResult(results[searchIndex]); }
    else if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); searchOpen = false; }
  }
  function pickResult(port: Port) {
    searchOpen = false;
    showOnShelf(port);
  }
  let config = $state<Record<string, Record<string, unknown>> | null>(null);
  let configTab = $state("");

  const isInstalled = (id: string) => !!library?.installed[id];
  const displayName = (port: Port) => library?.overrides?.[port.id]?.name ?? port.name;

  // Images are decoded before they are shown; otherwise WebKit decodes each one the first
  // time it scrolls into the carousel and the item flickers.
  const decode = (src: string) => {
    const img = new Image();
    img.src = src;
    return img.decode().catch(() => {});
  };
  async function loadCover(id: string) {
    const cover = await invoke<string | null>("get_cover", { id });
    if (cover) await decode(cover);
    covers[id] = cover;
  }
  for (const media of ["n64-cartridge", "snes-cartridge", "gba-cartridge", "md-cartridge", "gc-disc", "ps1-disc", "ps2-disc", "x360-disc"]) {
    decode(`/media/${media}.png`);
  }
  for (const id of ["n64", "gc", "snes", "gba", "ps1", "ps2", "md", "x360"]) decode(`/consoles/${id}.png`);

  // Fill in missing covers one at a time in the background (libretro-thumbnails).
  async function scrapeMissing() {
    if (!catalog) return;
    for (const port of catalog.ports) {
      if (covers[port.id]) continue;
      try {
        const found = await invoke<string | null>("scrape_cover", { id: port.id, console: port.console, title: port.title ?? port.name, force: false });
        if (found) await loadCover(port.id);
      } catch {
        // No box art for this one; it keeps its drawn label.
      }
    }
  }

  async function findCover(port: Port, title = displayName(port)) {
    try {
      await invoke("unlock_cover", { id: port.id });
      const found = await invoke<string | null>("scrape_cover", { id: port.id, console: port.console, title, force: true });
      await loadCover(port.id);
      flash(found ? `Cover: ${found.replace(/\.png$/, "")}` : "No cover found");
    } catch (e) {
      error = String(e);
    }
  }

  async function pickCover(port: Port) {
    const picked = await open({ title: `Cover for ${displayName(port)}`, filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "webp"] }] });
    if (typeof picked !== "string") return;
    try {
      await invoke("set_cover", { id: port.id, path: picked });
      await loadCover(port.id);
      library = await invoke("get_library");
    } catch (e) {
      error = String(e);
    }
  }

  // Renaming: the title in the caption becomes a text field.
  let editingName = $state(false);
  let nameDraft = $state("");
  function startRename() {
    if (!game) return;
    nameDraft = displayName(game);
    editingName = true;
  }
  async function saveName() {
    if (!game) return;
    const port = game;
    editingName = false;
    const custom = nameDraft.trim() === port.name ? null : nameDraft.trim();
    try {
      library = await invoke("rename", { id: port.id, name: custom });
      // A new name is also a better search term, unless the cover was picked by hand.
      if (custom && !library?.overrides?.[port.id]?.cover_locked) await findCover(port, custom);
    } catch (e) {
      error = String(e);
    }
  }

  const knownPorts = (consoleId: string) =>
    (catalog?.ports ?? []).filter((p) => p.console === consoleId).sort((a, b) => Number(isInstalled(b.id)) - Number(isInstalled(a.id)) || displayName(a).localeCompare(displayName(b)));
  const host = (url: string) => {
    try {
      const u = new URL(url);
      return u.hostname === "github.com" ? `github.com/${u.pathname.split("/").slice(1, 3).join("/")}` : u.hostname.replace(/^www\./, "");
    } catch {
      return url;
    }
  };

  /** Jump from the list to the port on the shelf (showing every port if it is not installed). */
  function showOnShelf(port: Port) {
    if (!isInstalled(port.id)) showAll = true;
    knownFor = null;
    systemIndex = Math.max(0, systems.findIndex((s) => s.id === port.console));
    gameIndex = Math.max(0, systems[systemIndex]?.ports.findIndex((p) => p.id === port.id) ?? 0);
    view = "games";
  }

  function flash(message: string) {
    status = message;
    setTimeout(() => (status = ""), 3000);
  }

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
  let primaryLabel = $derived(view === "systems" ? "Open" : game && isInstalled(game.id) && !romReady(game.id) ? "Select game file" : game && !isInstalled(game.id) ? "Get it" : "Play");

  async function load() {
    try {
      catalog = await invoke<Catalog>("get_catalog");
      library = await invoke<Library>("get_library");
      await Promise.all(catalog.ports.map((port) => loadCover(port.id)));
      scrapeMissing();
    } catch (e) {
      error = String(e);
    }
  }

  function move(delta: number) {
    if (panelOpen()) return;
    if (view === "systems") systemIndex = wrap(systemIndex + delta, systems.length);
    else if (system) gameIndex = clamp(gameIndex + delta, system.ports.length);
  }
  const clamp = (i: number, n: number) => Math.max(0, Math.min(n - 1, i));
  const wrap = (i: number, n: number) => (n ? ((i % n) + n) % n : 0);

  async function confirm() {
    if (panelOpen()) return;
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
    if (panelOpen()) return;
    showAll = !showAll;
    systemIndex = clamp(systemIndex, systems.length);
    gameIndex = 0;
  }

  // Back from the systems screen asks once, then quits on the second press.
  let quitArmed = $state(false);
  let quitTimer: ReturnType<typeof setTimeout> | undefined;
  function back() {
    if (knownFor) knownFor = null;
    else if (settingsOpen) settingsOpen = false;
    else if (view === "games") view = "systems";
    else if (quitArmed) invoke("quit");
    else {
      quitArmed = true;
      clearTimeout(quitTimer);
      quitTimer = setTimeout(() => (quitArmed = false), 2500);
    }
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
    inputMode = "keys";
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
      e.preventDefault();
      openSearch();
      return;
    }
    if (e.target instanceof HTMLInputElement || editingName || searchOpen) return;
    const actions: Record<string, () => void> = {
      ArrowLeft: () => move(-1), ArrowRight: () => move(1), a: () => move(-1), d: () => move(1),
      Enter: confirm, " ": confirm, Escape: back, Backspace: back, s: openSettings, Tab: toggleAll, k: () => system && !panelOpen() && (knownFor = system.id), r: () => view === "games" && startRename(),
    };
    const action = actions[e.key];
    if (action) {
      e.preventDefault();
      action();
    }
  }

  // Controller, read by the backend and forwarded as "pad" events:
  // D-pad / stick browse, A confirms, B goes back, Y settings, Select every known port,
  // Start opens search, LB / RB switch system while looking at games.
  function onPad(action: string) {
    inputMode = "pad";
    if (searchOpen) {
      if (action === "down") searchIndex = Math.min(searchIndex + 1, results.length - 1);
      else if (action === "up") searchIndex = Math.max(searchIndex - 1, 0);
      else if (action === "a" && results[searchIndex]) pickResult(results[searchIndex]);
      else if (action === "b" || action === "start") searchOpen = false;
      return;
    }
    if (settingsOpen && config) {
      const tabs = Object.keys(config);
      const i = tabs.indexOf(configTab);
      if (action === "left" || action === "lb") configTab = tabs[Math.max(0, i - 1)];
      else if (action === "right" || action === "rb") configTab = tabs[Math.min(tabs.length - 1, i + 1)];
      else if (action === "b" || action === "y") settingsOpen = false;
      return;
    }
    if (knownFor) {
      if (action === "b") knownFor = null;
      return;
    }
    const actions: Record<string, () => void> = {
      left: () => move(-1), right: () => move(1), a: confirm, b: back, y: openSettings,
      select: toggleAll, start: openSearch, x: () => system && (knownFor = system.id),
      lb: () => switchSystem(-1), rb: () => switchSystem(1),
    };
    actions[action]?.();
  }
  function switchSystem(delta: number) {
    if (view !== "games") return;
    systemIndex = wrap(systemIndex + delta, systems.length);
    gameIndex = 0;
  }
  listen<string>("pad", (e) => onPad(e.payload));

  const prettyKey = (k: string) => k.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
  const kindLabel = { recomp: "Static recompilation", decomp: "Decompilation", build: "Builds from your ROM" };

  // Position of item i relative to the focused one.
  const slot = (i: number, focused: number, spacing: number, loop = 0) => {
    let offset = i - focused;
    // On a looping carousel each item sits on whichever side of the focus is closer.
    if (loop > 2) offset = wrap(offset + Math.floor(loop / 2), loop) - Math.floor(loop / 2);
    const distance = Math.abs(offset);
    return `transform: translate(calc(-50% + ${offset * spacing}px), -50%) scale(${Math.max(0.55, 1 - distance * 0.18)}); opacity: ${Math.max(0, 1 - distance * 0.28)}; z-index: ${100 - distance};`;
  };

  load();
</script>

<svelte:window onkeydown={onKey} onmousedown={() => (inputMode = "keys")} onmousemove={(e) => { if (e.movementX || e.movementY) inputMode = "keys"; }} />

<main class:pad-mode={inputMode === "pad"} style="--accent: {system?.info.color ?? '#f2b04c'}">
  <header>
    <h1>PortShelf</h1>
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
        <button class="slot" class:focused={i === systemIndex} style={slot(i, systemIndex, 520, systems.length)} onclick={() => (i === systemIndex ? confirm() : (systemIndex = i))}>
          <ConsoleIcon id={s.id} size={460} />
        </button>
      {/each}
    </section>
    {#if system}
      <div class="caption">
        <h2>{system.info.name}</h2>
        <p>
          {system.info.year} · {system.installed} installed ·
          <button class="link inline" onclick={() => (knownFor = system!.id)}>{knownPorts(system.id).length} known</button>
        </p>
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
            <Disc name={displayName(port)} cover={covers[port.id]} console={port.console} installed={isInstalled(port.id)} size={2.2} />
          {:else}
            <Cartridge name={displayName(port)} cover={covers[port.id]} console={port.console} shell={port.shell} installed={isInstalled(port.id)} size={2.2} />
          {/if}
        </button>
      {/each}
    </section>
    {#if game}
      <div class="caption">
        <p class="system-name">{system.info.name}</p>
        {#if editingName}
          <!-- svelte-ignore a11y_autofocus -->
          <input class="rename" bind:value={nameDraft} autofocus
            onkeydown={(e) => { if (e.key === "Enter") saveName(); if (e.key === "Escape") { e.stopPropagation(); editingName = false; } }}
            onblur={saveName} aria-label="Game name" />
        {:else}
          <h2 class="title">
            {displayName(game)}
            <button class="icon" onclick={startRename} title="Rename (R)" aria-label="Rename">✎</button>
          </h2>
          {#if library?.overrides?.[game.id]?.name}<p class="original">{game.name}</p>{/if}
        {/if}
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
        <div class="actions small">
          <button class="link" onclick={() => pickCover(game!)}>Choose cover…</button>
          <button class="link" onclick={() => findCover(game!)}>Find cover online</button>
        </div>
        {#if status}<p class="status">{status}</p>{/if}
      </div>
    {/if}
   </div>
  {/if}
  </div>

  {#if quitArmed}
    <p class="quit-hint" role="status">Press {inputMode === "pad" ? "B" : "Esc"} again to quit PortShelf</p>
  {/if}
  <footer>
    {#if inputMode === "pad"}
      {#if view === "games"}<span><PadGlyph button="east" /> Systems</span>{/if}
      {#if view === "systems"}<span><PadGlyph button="east" /> Quit</span>{/if}
      <span><PadGlyph button="dpad" /> Browse</span>
      <span><PadGlyph button="south" /> {primaryLabel}</span>
      {#if view === "games"}<span><PadGlyph button="north" /> Settings</span>{/if}
      {#if view === "games"}<span><kbd class="pad">LB</kbd><kbd class="pad">RB</kbd> System</span>{/if}
      <span><PadGlyph button="west" /> Known ports</span>
      <span><kbd class="pad">Start</kbd> Search</span>
      <span><kbd class="pad">Select</kbd> {showAll ? "Installed only" : "Every known port"}</span>
    {:else}
      {#if view === "games"}<span><kbd>Esc</kbd> Systems</span>{/if}
      {#if view === "systems"}<span><kbd>Esc</kbd> Quit</span>{/if}
      <span><kbd>←</kbd><kbd>→</kbd> Browse</span>
      <span><kbd>Enter</kbd> {primaryLabel}</span>
      {#if view === "games"}<span><kbd>S</kbd> Settings</span>{/if}
      {#if view === "games"}<span><kbd>R</kbd> Rename</span>{/if}
      <span><kbd>K</kbd> Known ports</span>
      <span><kbd>Ctrl</kbd><kbd>F</kbd> Search</span>
      <span><kbd>Tab</kbd> {showAll ? "Installed only" : "Every known port"}</span>
    {/if}
  </footer>
</main>

{#if searchOpen && catalog}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="scrim" onclick={() => (searchOpen = false)}>
    <div class="search" role="dialog" aria-label="Search games" tabindex="-1" onclick={(e) => e.stopPropagation()}>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="query" type="search" placeholder="Search every system…" autofocus
        bind:value={query} oninput={() => (searchIndex = 0)} onkeydown={onSearchKey}
        aria-controls="search-results" aria-activedescendant={results[searchIndex] ? `result-${results[searchIndex].id}` : undefined}
      />
      <ul id="search-results" class="results" role="listbox">
        {#each results as port, i (port.id)}
          <li id="result-{port.id}" role="option" aria-selected={i === searchIndex}>
            <button class:on={i === searchIndex} onclick={() => pickResult(port)} onmouseenter={() => (searchIndex = i)}>
              <span class="thumb">{#if covers[port.id]}<img src={covers[port.id]} alt="" />{/if}</span>
              <span class="info">
                <span class="name">{displayName(port)}</span>
                <span class="meta">{catalog.consoles[port.console].name} · {kindLabel[port.kind]}{#if isInstalled(port.id)} · <span class="installed">installed</span>{/if}</span>
              </span>
            </button>
          </li>
        {:else}
          <li class="none">No game matches “{query}”.</li>
        {/each}
      </ul>
    </div>
  </div>
{/if}

{#if knownFor && catalog}
  <aside aria-label="Known {catalog.consoles[knownFor].name} ports">
    <button class="close ghost" onclick={() => (knownFor = null)} aria-label="Close">×</button>
    <h2>{catalog.consoles[knownFor].name}</h2>
    <p class="muted">Every port PortShelf knows about for this system. Links go to each project's page.</p>
    <ul class="known">
      {#each knownPorts(knownFor) as port (port.id)}
        <li>
          <button class="thumb" onclick={() => showOnShelf(port)} aria-label="Show {displayName(port)} on the shelf">
            {#if covers[port.id]}<img src={covers[port.id]} alt="" />{/if}
          </button>
          <div class="info">
            <button class="name" onclick={() => showOnShelf(port)}>{displayName(port)}</button>
            <span class="meta">{kindLabel[port.kind]}{#if isInstalled(port.id)} · <span class="installed">installed</span>{/if}</span>
            <button class="link" onclick={() => openUrl(port.repo)}>{host(port.repo)} ↗</button>
          </div>
        </li>
      {/each}
    </ul>
  </aside>
{/if}

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
  /* A launcher, not a document: nothing is selectable except text fields. */
  :global(*) { -webkit-user-select: none; user-select: none; }
  :global(input, textarea) { -webkit-user-select: text; user-select: text; }
  :global(img) { -webkit-user-drag: none; }
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
    will-change: transform, opacity;  /* own compositing layer: no repaint flicker while moving */
    background: none; border: 0; padding: 0;
    transition: transform 0.28s cubic-bezier(0.2, 0.8, 0.2, 1), opacity 0.28s ease;
  }
  .slot:focus-visible { outline: 2px solid #f2b04c; outline-offset: 8px; border-radius: 12px; }

  /* Focused items lift smoothly (a transition, so leaving focus never snaps back), and discs
     spin only while focused: pausing keeps their angle instead of jumping back to 0. */
  .slot > :global(*) { transition: translate 0.35s cubic-bezier(0.2, 0.8, 0.2, 1); }
  .slot.focused > :global(*) { translate: 0 -12px; }
  .slot :global(.disc) { animation: spin 9s linear infinite paused; }
  .slot.focused :global(.disc) { animation-play-state: running; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) {
    .slot, .slot > :global(*), .slot :global(.disc) { animation: none; transition: none; }
  }

  .caption { text-align: center; min-height: 170px; }
  .caption h2 { margin: 0; font-size: 28px; }
  .caption p { margin: 4px 0; color: #bdb4a7; }
  .system-name { text-transform: uppercase; letter-spacing: 2px; font-size: 12px; }
  .actions { display: flex; gap: 10px; justify-content: center; margin-top: 14px; flex-wrap: wrap; }
  .status { color: #9fd48b; }
  .title { display: inline-flex; align-items: center; gap: 8px; }
  .icon { background: none; border: 0; color: #8f877b; font-size: 18px; padding: 2px 4px; border-radius: 4px; }
  .icon:hover, .icon:focus-visible { color: #f2b04c; }
  .rename {
    font: 600 26px/1.2 system-ui, sans-serif; text-align: center; color: inherit;
    background: #17161b; border: 1px solid #f2b04c; border-radius: 6px; padding: 2px 10px; width: min(520px, 90%);
  }
  .original { font-size: 12px; }
  .actions.small { margin-top: 6px; gap: 16px; }
  .link { background: none; border: 0; color: #9b948a; font-size: 13px; text-decoration: underline; text-underline-offset: 3px; }
  .link:hover, .link:focus-visible { color: #f2b04c; }
  .rom { font-size: 13px; color: #9fd48b !important; }
  .rom.missing { color: #f2b04c !important; }

  footer { display: flex; gap: 22px; justify-content: center; color: #8f877b; font-size: 13px; padding: 8px 0 4px; flex-wrap: wrap; }
  footer span { display: inline-flex; align-items: center; gap: 4px; }
  kbd.pad { border-radius: 10px; min-width: 22px; font-weight: 700; }
  /* Using the controller: no mouse pointer until the mouse moves again. */
  :global(body:has(main.pad-mode)), :global(body:has(main.pad-mode) *) { cursor: none !important; }
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
  .quit-hint {
    position: fixed; left: 50%; bottom: 64px; transform: translateX(-50%); margin: 0;
    background: #2c2a33; border: 1px solid #4a4540; border-radius: 8px; padding: 8px 16px; color: #eee8df;
  }
  .scrim { position: fixed; inset: 0; background: rgba(8, 8, 10, 0.6); display: grid; place-items: start center; padding-top: 12vh; z-index: 200; }
  .search { width: min(640px, 92vw); background: #1b1a1f; border: 1px solid #333; border-radius: 12px; box-shadow: 0 24px 60px rgba(0,0,0,0.6); overflow: hidden; }
  .query { width: 100%; box-sizing: border-box; border: 0; border-bottom: 1px solid #333; background: transparent; color: inherit; font: 18px system-ui, sans-serif; padding: 16px 18px; outline: none; }
  .results { list-style: none; margin: 0; padding: 6px; max-height: 56vh; overflow-y: auto; }
  .results button { width: 100%; display: flex; gap: 12px; align-items: center; background: none; border: 0; border-radius: 8px; padding: 6px 8px; text-align: left; }
  .results button.on { background: #2c2a33; }
  .results .thumb { width: 56px; height: 40px; flex: none; border-radius: 4px; overflow: hidden; background: #2d2c33; }
  .results .thumb img { width: 100%; height: 100%; object-fit: cover; }
  .results .name { display: block; font-weight: 600; }
  .results .meta { display: block; font-size: 12px; color: #9b948a; }
  .results .installed { color: #9fd48b; }
  .results .none { padding: 14px; color: #9b948a; }
  .link.inline { font-size: inherit; padding: 0; color: #d6cec2; }
  .known { list-style: none; padding: 0; margin: 12px 0 0; display: grid; gap: 10px; }
  .known li { display: flex; gap: 12px; align-items: center; padding: 8px; border-radius: 8px; background: #222127; }
  .thumb { width: 64px; height: 46px; flex: none; padding: 0; border: 0; border-radius: 4px; overflow: hidden; background: #2d2c33; }
  .thumb img { width: 100%; height: 100%; object-fit: cover; }
  .info { display: flex; flex-direction: column; align-items: flex-start; gap: 2px; min-width: 0; }
  .info .name { background: none; border: 0; padding: 0; font-weight: 600; text-align: left; }
  .info .name:hover, .info .name:focus-visible { color: #f2b04c; }
  .info .meta { font-size: 12px; color: #9b948a; }
  .info .installed { color: #9fd48b; }
  .info .link { padding: 0; font-size: 12px; }
</style>
