<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import Cartridge from "$lib/Cartridge.svelte";
  import Disc from "$lib/Disc.svelte";
  import ConsoleIcon from "$lib/ConsoleIcon.svelte";
  import PadGlyph from "$lib/PadGlyph.svelte";
  import SystemLogo from "$lib/SystemLogo.svelte";
  import { applyTheme, themes, type Mode } from "$lib/themes";
  import { defaultLang, languages, translate, type Key, type Lang } from "$lib/i18n";
  import { open } from "@tauri-apps/plugin-dialog";
  import { fade, fly, scale } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import type { Catalog, Library, Port, RomStatus } from "$lib/types";

  // Theme and light / dark mode, remembered on this machine.
  const stored = (key: string) => {
    try {
      return localStorage.getItem(key);
    } catch {
      return null;
    }
  };
  let themeId = $state(stored("portshelf.theme") ?? "arcade");
  let mode = $state<Mode>((stored("portshelf.mode") as Mode) ?? "dark");
  let lang = $state<Lang>((stored("portshelf.lang") as Lang) ?? defaultLang());
  /** Interface text in the chosen language. */
  const tr = (key: Key, vars: Record<string, string | number> = {}) => translate(lang, key, vars);
  $effect(() => {
    applyTheme(themeId, mode);
    try {
      localStorage.setItem("portshelf.theme", themeId);
      localStorage.setItem("portshelf.mode", mode);
      localStorage.setItem("portshelf.lang", lang);
    } catch {
      // not persisted; the theme still applies for this session
    }
  });

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
  const panelOpen = () => settingsOpen || knownFor !== null || searchOpen || quitOpen || addPort !== null || optionsOpen;
  // App settings dialog: appearance, language, ROM folder, adding ports.
  let optionsOpen = $state(false);
  const systemRomStatus = (consoleId: string) => {
    const installed = (catalog?.ports ?? []).filter((p) => p.console === consoleId && isInstalled(p.id));
    return { ready: installed.filter((p) => romReady(p.id)).length, installed: installed.length };
  };

  // ROM folder: <root>/<system>/<game>/, created on first choice and synced on start and Rescan.
  type RomSummary = { ready: number; installed: number; assigned: number };
  let romSummary = $state<RomSummary | null>(null);
  let syncing = $state(false);
  async function syncRoms() {
    if (!library?.roms_dir) return;
    syncing = true;
    try {
      romSummary = await invoke<RomSummary>("sync_roms");
      library = await invoke("get_library");
      if (game) await refreshRom(game);
      if (romSummary.assigned) flash(tr("msg.filesFound", { count: romSummary.assigned }));
    } catch (e) {
      error = String(e);
    } finally {
      syncing = false;
    }
  }
  async function chooseRomFolder() {
    const dir = await open({ directory: true, title: tr("pick.romFolder"), defaultPath: library?.roms_dir || undefined });
    if (typeof dir !== "string") return;
    syncing = true;
    try {
      romSummary = await invoke<RomSummary>("set_roms_dir", { dir });
      library = await invoke("get_library");
      if (game) await refreshRom(game);
      flash(tr("msg.romFolderReady"));
    } catch (e) {
      error = String(e);
    } finally {
      syncing = false;
    }
  }
  // The ROM button opens the system's file chooser inside the system's folder, where every
  // game has its own folder; the picked file goes to the game whose folder it is in.
  async function romFolderAction(consoleId: string | undefined = system?.id) {
    if (!library?.roms_dir) return chooseRomFolder();
    const picked = await open({
      title: consoleId ? tr("pick.systemFiles", { system: catalog?.consoles[consoleId].name ?? "" }) : tr("pick.gameFiles"),
      defaultPath: consoleId ? `${library.roms_dir}/${consoleId}` : library.roms_dir,
    });
    if (typeof picked !== "string") return;
    try {
      const ids = await invoke<string[]>("assign_rom", { path: picked });
      library = await invoke("get_library");
      const names = ids.map((id) => catalog?.ports.find((p) => p.id === id)).filter((p): p is Port => !!p);
      await Promise.all(names.filter((p) => isInstalled(p.id)).map(refreshRom));
      flash(tr("msg.fileSetFor", { names: names.map(displayName).join(` ${tr("game.and")} `) }));
    } catch (e) {
      error = String(e);
    }
  }

  let systemRoms = $derived.by(() => {
    if (!system) return null;
    const installed = system.ports.filter((p) => isInstalled(p.id));
    return { ready: installed.filter((p) => romReady(p.id)).length, installed: installed.length };
  });

  // Ports and game files are picked up automatically: on start and whenever the window
  // comes back into focus (after dropping a file in the ROM folder, for example).
  let lastRefresh = 0;
  async function refreshAll() {
    if (Date.now() - lastRefresh < 3000 || !catalog) return;
    lastRefresh = Date.now();
    library = await invoke("rescan");
    await syncRoms();
    await Promise.all(catalog.ports.filter((p) => isInstalled(p.id)).map(refreshRom));
  }
  getCurrentWindow().onFocusChanged(({ payload: focused }) => {
    if (focused) refreshAll();
  });

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

  // Installing ports from their GitHub releases (the game file still comes from the user).
  let os = $state("linux");
  invoke<string>("platform").then((p) => (os = p));
  const canInstall = (port: Port) => !!port.install?.[os as "linux" | "windows" | "macos"];
  /** Not made for this operating system (for example, Windows only). */
  const unavailable = (port: Port) => !!port.platforms && !port.platforms.includes(os);
  const platformLabel = (port: Port) =>
    tr("game.onlyOn", { platforms: (port.platforms ?? []).map((p) => ({ windows: "Windows", linux: "Linux", macos: "macOS" })[p] ?? p).join(` ${tr("game.and")} `) });
  /** Ports that ask for their game file themselves (PortShelf does not know where they keep it). */
  const selfManaged = (id: string) => isInstalled(id) && !library?.installed[id]?.rom;
  // "Add port": pick a port's program; PortShelf guesses which catalog port it is from the
  // file and folder names, and the user confirms, picks another, or describes a new one.
  let addPort = $state<{ exec: string; choice: string; name: string; console: string } | null>(null);
  const squash = (t: string) => t.toLowerCase().replace(/[^a-z0-9]/g, "");
  function guessPort(exec: string): string {
    if (!catalog) return "new";
    const parts = exec.split("/");
    const file = squash((parts.pop() ?? "").replace(/\.(appimage|exe|x86_64)$/i, ""));
    const folder = squash(parts.pop() ?? "");
    let best = { id: "new", score: 0 };
    for (const port of catalog.ports) {
      const words = [port.id, port.name, port.title ?? "", port.repo.split("/").filter(Boolean).pop() ?? "", ...port.id.split("-")]
        .map(squash)
        .filter((w) => w.length >= 3);
      for (const hay of [file, folder]) {
        if (!hay) continue;
        for (const w of words) {
          const score = hay.includes(w) ? w.length : w.includes(hay) && hay.length >= 4 ? hay.length : 0;
          const bonus = isInstalled(port.id) ? 0 : 0.5;
          if (score && score + bonus > best.score) best = { id: port.id, score: score + bonus };
        }
      }
    }
    return best.id;
  }
  async function startAddPort() {
    optionsOpen = false;
    if (panelOpen()) return;
    const exec = await open({ title: tr("pick.program") });
    if (typeof exec !== "string") return;
    const choice = guessPort(exec);
    addPort = { exec, choice, name: "", console: system?.id ?? "n64" };
  }
  async function confirmAddPort() {
    if (!addPort) return;
    const { exec, choice, name, console: consoleId } = addPort;
    try {
      const id = choice === "new" ? await invoke<string>("add_custom_port", { name, console: consoleId }) : choice;
      library = await invoke("add_install", { id, exec });
      catalog = await invoke<Catalog>("get_catalog");
      addPort = null;
      const port = catalog.ports.find((p) => p.id === id);
      if (port) {
        await refreshRom(port);
        showOnShelf(port);
        flash(tr("msg.added", { name: displayName(port) }));
      }
    } catch (e) {
      error = String(e);
    }
  }
  let installing = $state<Record<string, { stage: string; done: number; total: number | null }>>({});
  listen<{ id: string; stage: string; done: number; total: number | null }>("install-progress", (e) => {
    installing[e.payload.id] = e.payload;
  });
  const mb = (n: number) => (n / 1048576).toFixed(0);
  const installLabel = (id: string) => {
    const p = installing[id];
    if (!p) return "";
    if (p.stage === "downloading") return p.total ? tr("game.downloading", { done: mb(p.done), total: mb(p.total) }) : tr("game.downloadingNoTotal", { done: mb(p.done) });
    return p.stage === "unpacking" ? tr("game.unpacking") : tr("game.finishing");
  };
  async function installPort(port: Port) {
    if (installing[port.id]) return;
    installing[port.id] = { stage: "downloading", done: 0, total: null };
    try {
      library = await invoke("install_port", { id: port.id });
      delete installing[port.id];
      // Installed ports sort first; keep the focus on the one just installed.
      const at = systems[systemIndex]?.ports.findIndex((p) => p.id === port.id) ?? -1;
      if (at >= 0) gameIndex = at;
      await refreshRom(port);
      flash(tr(roms[port.id]?.ready ? "msg.installed" : "msg.installedNeedsFile", { name: displayName(port) }));
    } catch (e) {
      delete installing[port.id];
      error = String(e);
    }
  }
  /** Catalog authors, or the GitHub owner of the project. */
  const authorsOf = (port: Port) => port.authors ?? port.repo.match(/^https:\/\/github\.com\/([^/]+)/)?.[1] ?? "";
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
      flash(found ? tr("msg.cover", { file: found.replace(/\.png$/, "") }) : tr("msg.noCover"));
    } catch (e) {
      error = String(e);
    }
  }

  async function pickCover(port: Port) {
    const picked = await open({ title: tr("pick.cover", { name: displayName(port) }), filters: [{ name: tr("pick.images"), extensions: ["png", "jpg", "jpeg", "webp"] }] });
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
      title: tr("pick.gameFile", { system: catalog?.consoles[port.console].name ?? "", name: displayName(port) }),
      defaultPath: roms[port.id]?.browse_dir,
      filters: [{ name: tr("pick.gameFiles"), extensions }],
    });
    if (typeof picked !== "string") return;
    try {
      await invoke("select_rom", { id: port.id, path: picked });
      await refreshRom(port);
      status = tr("msg.fileReady");
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
  let primaryLabel = $derived(
    view === "systems" ? tr("game.open")
    : !game ? ""
    : isInstalled(game.id) ? (romReady(game.id) ? (selfManaged(game.id) ? tr("game.start") : tr("game.play")) : tr("game.selectFile"))
    : unavailable(game) ? tr("game.projectPage")
    : canInstall(game) ? tr("game.install") : tr("game.getIt"));

  async function load() {
    try {
      catalog = await invoke<Catalog>("get_catalog");
      library = await invoke<Library>("get_library");
      await Promise.all(catalog.ports.map((port) => loadCover(port.id)));
      scrapeMissing();
      await syncRoms();
      await Promise.all(catalog.ports.filter((p) => isInstalled(p.id)).map(refreshRom));
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
      if (!isInstalled(game.id)) {
        if (unavailable(game)) openUrl(game.repo);
        else if (canInstall(game)) await installPort(game);
        else openUrl(game.repo);
      }
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

  // Back on the systems screen asks whether to quit, in a yes / no dialog ("No" focused).
  let quitOpen = $state(false);
  let quitYes = $state(false);
  function back() {
    if (optionsOpen) optionsOpen = false;
    else if (addPort) addPort = null;
    else if (knownFor) knownFor = null;
    else if (settingsOpen) settingsOpen = false;
    else if (view === "games") view = "systems";
    else {
      quitYes = false;
      quitOpen = true;
    }
  }
  function answerQuit(yes: boolean) {
    quitOpen = false;
    if (yes) invoke("quit");
  }

  async function play(port: Port) {
    try {
      await invoke("launch", { id: port.id });
      status = tr("msg.started", { name: displayName(port) });
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
    if (quitOpen) {
      e.preventDefault();
      if (["ArrowLeft", "ArrowRight", "a", "d", "Tab"].includes(e.key)) quitYes = !quitYes;
      else if (e.key === "Enter" || e.key === " ") answerQuit(quitYes);
      else if (e.key === "Escape" || e.key === "Backspace") answerQuit(false);
      else if (e.key.toLowerCase() === "y") answerQuit(true);
      else if (e.key.toLowerCase() === "n") answerQuit(false);
      return;
    }
    if (e.target instanceof HTMLInputElement || editingName || searchOpen) return;
    const actions: Record<string, () => void> = {
      ArrowLeft: () => move(-1), ArrowRight: () => move(1), a: () => move(-1), d: () => move(1),
      Enter: confirm, " ": confirm, Escape: back, Backspace: back, s: openSettings, i: () => romFolderAction(), p: startAddPort, o: () => !panelOpen() && (optionsOpen = true), Tab: toggleAll,
      t: () => { const ids = Object.keys(themes); themeId = ids[(ids.indexOf(themeId) + 1) % ids.length]; flash(tr("msg.theme", { name: themes[themeId].name })); },
      m: () => (mode = mode === "dark" ? "light" : "dark"),
      l: () => (lang = lang === "en" ? "pt-BR" : "en"), k: () => system && !panelOpen() && (knownFor = system.id), r: () => view === "games" && startRename(),
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
    if (quitOpen) {
      if (action === "left" || action === "right") quitYes = !quitYes;
      else if (action === "a") answerQuit(quitYes);
      else if (action === "b") answerQuit(false);
      return;
    }
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
    if (addPort) {
      if (action === "b") addPort = null;
      return;
    }
    if (optionsOpen) {
      if (action === "b" || action === "y") optionsOpen = false;
      return;
    }
    const actions: Record<string, () => void> = {
      left: () => move(-1), right: () => move(1), a: confirm, b: back,
      y: () => (view === "systems" ? (optionsOpen = true) : openSettings()),
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
  const kindLabel = (kind: string) => tr(`kind.${kind}` as Key);

  // Mouse: drag the carousel sideways (or use the wheel) to browse. A drag of about one
  // item's width moves one step; a drag never counts as a click on the item under it.
  let drag: { x: number; moved: boolean; step: number } | null = null;
  let suppressClick = false;
  function dragStart(e: PointerEvent, step: number) {
    if (e.button !== 0) return;
    drag = { x: e.clientX, moved: false, step };
  }
  function dragMove(e: PointerEvent) {
    if (!drag) return;
    const dx = e.clientX - drag.x;
    if (Math.abs(dx) >= drag.step * 0.5) {
      move(dx < 0 ? 1 : -1);
      drag.x = e.clientX;
      drag.moved = true;
    }
  }
  function dragEnd() {
    if (drag?.moved) {
      suppressClick = true;
      setTimeout(() => (suppressClick = false), 0);
    }
    drag = null;
  }
  let wheelAt = 0;
  function onWheel(e: WheelEvent) {
    const delta = Math.abs(e.deltaX) > Math.abs(e.deltaY) ? e.deltaX : e.deltaY;
    if (Math.abs(delta) < 4 || Date.now() - wheelAt < 120) return;
    wheelAt = Date.now();
    move(delta > 0 ? 1 : -1);
  }
  const clickSlot = (i: number, focused: number, select: (i: number) => void) => {
    if (suppressClick) return;
    if (i === focused) confirm();
    else select(i);
  };

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

<main class:pad-mode={inputMode === "pad"} style="--system: {system?.info.color ?? 'var(--accent)'}">
  <header>
    <h1 class="brand"><img src="/icon.svg" alt="" width="34" height="34" draggable="false" />PortShelf</h1>
    <button class="tool square gear" onclick={() => (optionsOpen = true)} title={tr("settings.open")} aria-label={tr("settings.open")}>⚙</button>
  </header>

  {#if error}
    <p class="error" role="alert">{error} <button class="ghost" onclick={() => (error = "")}>{tr("common.dismiss")}</button></p>
  {/if}

  <div class="stage">
  {#if view === "systems"}
   <div class="view" in:fade={{ duration: 220, delay: 120 }} out:scale={{ start: 1.6, opacity: 0, duration: 260, easing: cubicOut }}>
    <section class="carousel systems" aria-label={tr("systems.aria")}
      onpointerdown={(e) => dragStart(e, 520)} onpointermove={dragMove} onpointerup={dragEnd} onpointerleave={dragEnd} onwheel={onWheel}>
      {#each systems as s, i (s.id)}
        <button class="slot" class:focused={i === systemIndex} style={slot(i, systemIndex, 520, systems.length)} onclick={() => clickSlot(i, systemIndex, (j) => (systemIndex = j))}>
          <ConsoleIcon id={s.id} size={460} />
        </button>
      {/each}
    </section>
    {#if system}
      <div class="caption">
        <h2 class="system-logo"><SystemLogo id={system.id} name={system.info.name} height={52} /></h2>
        <p>
          {system.info.year} · {tr("systems.installed", { count: system.installed })} ·
          <button class="link inline" onclick={() => (knownFor = system!.id)}>{tr("systems.known", { count: knownPorts(system.id).length })}</button>
        </p>
      </div>
    {/if}
   </div>
  {:else if system}
   <div class="view" out:fade={{ duration: 160 }}>
    <section class="carousel games" aria-label={system.info.name}
      onpointerdown={(e) => dragStart(e, 440)} onpointermove={dragMove} onpointerup={dragEnd} onpointerleave={dragEnd} onwheel={onWheel}>
      {#each system.ports as port, i (port.id)}
        <button class="slot" class:focused={i === gameIndex} style={slot(i, gameIndex, 440)} onclick={() => clickSlot(i, gameIndex, (j) => (gameIndex = j))}
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
        <p class="system-name"><SystemLogo id={system.id} name={system.info.name} height={22} /></p>
        {#if editingName}
          <!-- svelte-ignore a11y_autofocus -->
          <input class="rename" bind:value={nameDraft} autofocus
            onkeydown={(e) => { if (e.key === "Enter") saveName(); if (e.key === "Escape") { e.stopPropagation(); editingName = false; } }}
            onblur={saveName} aria-label={tr("game.nameField")} />
        {:else}
          <h2 class="title">
            {displayName(game)}
            <button class="icon" onclick={startRename} title={tr("game.renameHint")} aria-label={tr("game.rename")}>✎</button>
          </h2>
          {#if library?.overrides?.[game.id]?.name}<p class="original">{game.name}</p>{/if}
        {/if}
        <p>{kindLabel(game.kind)}{#if authorsOf(game)} · {tr("game.by", { authors: authorsOf(game) })}{/if}{isInstalled(game.id) ? "" : ` · ${tr("game.notInstalled")}`}</p>
        {#if selfManaged(game.id)}
          <p class="rom note">{tr("game.asksForFile")}</p>
        {:else if isInstalled(game.id) && roms[game.id] && !romReady(game.id)}
          <p class="rom missing">{tr("game.missingFile")}</p>
        {/if}
        <div class="actions">
          {#if isInstalled(game.id) && romReady(game.id)}
            <button class="primary" onclick={() => play(game!)}>{selfManaged(game.id) ? tr("game.start") : tr("game.play")}</button>
            <button class="ghost" onclick={() => selectRom(game!)}>{tr("game.changeFile")}</button>
            <button class="ghost" onclick={openSettings}>{tr("game.settings")}</button>
          {:else if isInstalled(game.id)}
            <button class="primary" onclick={() => selectRom(game!)}>{tr("game.selectFile")}</button>
            <button class="ghost" onclick={openSettings}>{tr("game.settings")}</button>
          {:else if installing[game.id]}
            <button class="primary progress" disabled style="--p:{installing[game.id].total ? (installing[game.id].done / installing[game.id].total!) * 100 : 0}%">{installLabel(game.id)}</button>
          {:else if unavailable(game)}
            <button class="primary" disabled>{platformLabel(game)}</button>
          {:else if canInstall(game)}
            <button class="primary" onclick={() => installPort(game!)}>{tr("game.install")}</button>
          {:else}
            <button class="primary" onclick={() => openUrl(game!.repo)}>{tr("game.getIt")}</button>
          {/if}
          <button class="ghost" onclick={() => openUrl(game!.repo)}>{tr("game.projectPage")}</button>
        </div>
        <div class="actions small">
          <button class="link" onclick={() => pickCover(game!)}>{tr("game.chooseCover")}</button>
          <button class="link" onclick={() => findCover(game!)}>{tr("game.findCover")}</button>
        </div>
        {#if status}<p class="status">{status}</p>{/if}
      </div>
    {/if}
   </div>
  {/if}
  </div>

  <footer>
    {#if inputMode === "pad"}
      {#if view === "games"}<span><PadGlyph button="east" /> {tr("footer.systems")}</span>{/if}
      {#if view === "systems"}<span><PadGlyph button="east" /> {tr("footer.quit")}</span><span><PadGlyph button="north" /> {tr("footer.options")}</span>{/if}
      <span><PadGlyph button="dpad" /> {tr("footer.browse")}</span>
      <span><PadGlyph button="south" /> {primaryLabel}</span>
      {#if view === "games"}<span><PadGlyph button="north" /> {tr("footer.settings")}</span>{/if}
      {#if view === "games"}<span><kbd class="pad">LB</kbd><kbd class="pad">RB</kbd> {tr("footer.system")}</span>{/if}
      <span><PadGlyph button="west" /> {tr("footer.known")}</span>
      <span><kbd class="pad">Start</kbd> {tr("footer.search")}</span>
      <span><kbd class="pad">Select</kbd> {showAll ? tr("footer.installedOnly") : tr("footer.everyPort")}</span>
    {:else}
      {#if view === "games"}<span><kbd>Esc</kbd> {tr("footer.systems")}</span>{/if}
      {#if view === "systems"}<span><kbd>Esc</kbd> {tr("footer.quit")}</span><span><kbd>O</kbd> {tr("footer.options")}</span>{/if}
      <span><kbd>←</kbd><kbd>→</kbd> {tr("footer.browse")}</span>
      <span><kbd>Enter</kbd> {primaryLabel}</span>
      {#if view === "games"}<span><kbd>S</kbd> {tr("footer.settings")}</span>{/if}
      {#if view === "games"}<span><kbd>R</kbd> {tr("footer.rename")}</span>{/if}
      <span><kbd>K</kbd> {tr("footer.known")}</span>
      <span><kbd>Ctrl</kbd><kbd>F</kbd> {tr("footer.search")}</span>
      <span><kbd>Tab</kbd> {showAll ? tr("footer.installedOnly") : tr("footer.everyPort")}</span>
    {/if}
  </footer>
</main>

{#if quitOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="scrim center" onclick={() => answerQuit(false)}>
    <div class="dialog" role="alertdialog" aria-modal="true" aria-labelledby="quit-title" tabindex="-1" onclick={(e) => e.stopPropagation()}>
      <h2 id="quit-title">{tr("quit.title")}</h2>
      <div class="choices">
        <button class:on={!quitYes} onclick={() => answerQuit(false)} onmouseenter={() => (quitYes = false)}>{tr("common.no")}</button>
        <button class:on={quitYes} onclick={() => answerQuit(true)} onmouseenter={() => (quitYes = true)}>{tr("common.yes")}</button>
      </div>
      <p class="dialog-hints">
        {#if inputMode === "pad"}
          <span><PadGlyph button="dpad" /> {tr("footer.choose")}</span>
          <span><PadGlyph button="south" /> {tr("footer.confirm")}</span>
          <span><PadGlyph button="east" /> {tr("footer.cancel")}</span>
        {:else}
          <span><kbd>←</kbd><kbd>→</kbd> {tr("footer.choose")}</span>
          <span><kbd>Enter</kbd> {tr("footer.confirm")}</span>
          <span><kbd>Esc</kbd> {tr("footer.cancel")}</span>
        {/if}
      </p>
    </div>
  </div>
{/if}

{#if optionsOpen && catalog}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="scrim center" onclick={() => (optionsOpen = false)}>
    <div class="dialog options" role="dialog" aria-modal="true" aria-labelledby="options-title" tabindex="-1" onclick={(e) => e.stopPropagation()}>
      <button class="close ghost" onclick={() => (optionsOpen = false)} aria-label={tr("common.close")}>×</button>
      <h2 id="options-title">{tr("settings.title")}</h2>

      <section>
        <h3>{tr("settings.appearance")}</h3>
        <div class="row">
          <label class="field">
            <span>{tr("header.theme")}</span>
            <select bind:value={themeId}>
              {#each Object.entries(themes) as [id, theme]}
                <option value={id}>{theme.name}</option>
              {/each}
            </select>
            <small>{tr(`theme.${themeId}` as Key)}</small>
          </label>
          <div class="field">
            <span>{tr("settings.mode")}</span>
            <div class="segmented" role="radiogroup" aria-label={tr("settings.mode")}>
              <button role="radio" aria-checked={mode === "dark"} class:on={mode === "dark"} onclick={() => (mode = "dark")}>☾ {tr("settings.dark")}</button>
              <button role="radio" aria-checked={mode === "light"} class:on={mode === "light"} onclick={() => (mode = "light")}>☀ {tr("settings.light")}</button>
            </div>
          </div>
          <label class="field">
            <span>{tr("header.language")}</span>
            <select bind:value={lang}>
              {#each Object.entries(languages) as [id, label]}
                <option value={id}>{label}</option>
              {/each}
            </select>
          </label>
        </div>
      </section>

      <section>
        <h3>{tr("settings.roms")}</h3>
        <div class="folder">
          <div>
            <span class="label">{tr("settings.romFolder")}</span>
            <code>{library?.roms_dir || tr("settings.noFolder")}</code>
          </div>
          <button class="tool" onclick={chooseRomFolder}>{library?.roms_dir ? tr("settings.changeFolder") : tr("header.chooseRomFolder")}</button>
        </div>
        {#if library?.roms_dir}
          <ul class="systems-roms">
            {#each Object.entries(catalog.consoles).sort((a, b) => a[1].year - b[1].year) as [cid, info] (cid)}
              {@const st = systemRomStatus(cid)}
              <li>
                <SystemLogo id={cid} name={info.name} height={22} />
                <span class="status" class:done={st.installed > 0 && st.ready === st.installed} class:warn={st.ready < st.installed}>
                  {st.installed ? tr("settings.readyOf", { ready: st.ready, installed: st.installed }) : tr("settings.noneInstalled")}
                </span>
                <button class="tool small" onclick={() => romFolderAction(cid)}>{tr("settings.chooseFile")}</button>
              </li>
            {/each}
          </ul>
        {/if}
      </section>

      <section>
        <h3>{tr("settings.ports")}</h3>
        <div class="folder">
          <p class="muted">{tr("settings.addPortHint")}</p>
          <button class="tool" onclick={startAddPort}>{tr("header.addPort")}</button>
        </div>
      </section>
    </div>
  </div>
{/if}

{#if addPort && catalog}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="scrim center" onclick={() => (addPort = null)}>
    <div class="dialog add-port" role="dialog" aria-modal="true" aria-labelledby="add-title" tabindex="-1" onclick={(e) => e.stopPropagation()}>
      <h2 id="add-title">{tr("add.title")}</h2>
      <p class="muted file">{addPort.exec}</p>
      <label class="field">
        <span>{tr("add.thisIs")}</span>
        <select bind:value={addPort.choice}>
          {#each Object.entries(catalog.consoles).sort((a, b) => a[1].year - b[1].year) as [cid, info]}
            <optgroup label={info.name}>
              {#each catalog.ports.filter((p) => p.console === cid) as port (port.id)}
                <option value={port.id}>{displayName(port)}{isInstalled(port.id) ? ` ${tr("add.installed")}` : ""}</option>
              {/each}
            </optgroup>
          {/each}
          <option value="new">{tr("add.new")}</option>
        </select>
      </label>
      {#if addPort.choice === "new"}
        <label class="field">
          <span>{tr("add.name")}</span>
          <!-- svelte-ignore a11y_autofocus -->
          <input type="text" bind:value={addPort.name} placeholder={tr("game.nameField")} autofocus />
        </label>
        <label class="field">
          <span>{tr("add.system")}</span>
          <select bind:value={addPort.console}>
            {#each Object.entries(catalog.consoles).sort((a, b) => a[1].year - b[1].year) as [cid, info]}
              <option value={cid}>{info.name}</option>
            {/each}
          </select>
        </label>
      {:else if isInstalled(addPort.choice)}
        <p class="warn-text">{tr("add.replaces")}</p>
      {/if}
      <div class="choices">
        <button onclick={() => (addPort = null)}>{tr("common.cancel")}</button>
        <button class="on" onclick={confirmAddPort} disabled={addPort.choice === "new" && !addPort.name.trim()}>{tr("common.add")}</button>
      </div>
    </div>
  </div>
{/if}

{#if searchOpen && catalog}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="scrim" onclick={() => (searchOpen = false)}>
    <div class="search" role="dialog" aria-label={tr("search.aria")} tabindex="-1" onclick={(e) => e.stopPropagation()}>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="query" type="search" placeholder={tr("search.placeholder")} autofocus
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
                <span class="meta">{catalog.consoles[port.console].name} · {kindLabel(port.kind)}{#if isInstalled(port.id)} · <span class="installed">{tr("search.installed")}</span>{/if}</span>
              </span>
            </button>
          </li>
        {:else}
          <li class="none">{tr("search.none", { query })}</li>
        {/each}
      </ul>
    </div>
  </div>
{/if}

{#if knownFor && catalog}
  <aside aria-label={tr("known.aria", { system: catalog.consoles[knownFor].name })}>
    <button class="close ghost" onclick={() => (knownFor = null)} aria-label={tr("common.close")}>×</button>
    <h2>{catalog.consoles[knownFor].name}</h2>
    <p class="muted">{tr("known.intro")}</p>
    <ul class="known">
      {#each knownPorts(knownFor) as port (port.id)}
        <li>
          <button class="thumb" onclick={() => showOnShelf(port)} aria-label={tr("known.show", { name: displayName(port) })}>
            {#if covers[port.id]}<img src={covers[port.id]} alt="" />{/if}
          </button>
          <div class="info">
            <button class="name" onclick={() => showOnShelf(port)}>{displayName(port)}</button>
            <span class="meta">{kindLabel(port.kind)}{#if isInstalled(port.id)} · <span class="installed">{tr("search.installed")}</span>{/if}</span>
            <button class="link" onclick={() => openUrl(port.repo)}>{host(port.repo)} ↗</button>
          </div>
        </li>
      {/each}
    </ul>
  </aside>
{/if}

{#if settingsOpen && config && game}
  <aside aria-label={tr("settings.aria", { name: displayName(game) })}>
    <button class="close ghost" onclick={() => (settingsOpen = false)} aria-label={tr("common.close")}>×</button>
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
      <p class="muted">{tr("settings.none")}</p>
    {/if}
  </aside>
{/if}

<style>
  /* A launcher, not a document: nothing is selectable except text fields. */
  :global(*) { -webkit-user-select: none; user-select: none; }
  :global(input, textarea) { -webkit-user-select: text; user-select: text; }
  :global(img) { -webkit-user-drag: none; }
  @font-face {
    font-family: "Outfit";
    src: url("/fonts/outfit.woff2") format("woff2");
    font-weight: 300 800;
    font-display: block;
  }
  @font-face {
    font-family: "Atkinson Hyperlegible Next";
    src: url("/fonts/atkinson-next.woff2") format("woff2");
    font-weight: 200 800;
    font-display: block;
  }
  :global(body) {
    margin: 0;
    background: var(--bg);
    color: var(--text);
    font: 16px/1.45 "Atkinson Hyperlegible Next", system-ui, sans-serif;
    overflow: hidden;
  }
  main {
    height: 100vh; display: flex; flex-direction: column; box-sizing: border-box; padding: 16px 28px;
    background: radial-gradient(ellipse at 50% 55%, color-mix(in srgb, var(--system) 30%, transparent), transparent 65%), var(--bg);
    transition: background 0.4s ease;
  }
  header { display: flex; align-items: center; gap: 20px; flex-wrap: wrap; }
  h1, h2, .brand, .caption h2, .dialog h2 { font-family: "Outfit", system-ui, sans-serif; }
  h1 { font-size: 22px; letter-spacing: 0.5px; margin: 4px 0; flex: 1; }
  .brand { display: flex; align-items: center; gap: 10px; font-weight: 800; letter-spacing: 0.08em; text-transform: uppercase; }
  .brand img { display: block; }
  .carousel { touch-action: pan-y; cursor: grab; }
  .carousel:active { cursor: grabbing; }
  button { font: inherit; color: inherit; cursor: pointer; }
  :global(input) { font-family: inherit; }
  .ghost { background: none; border: 1px solid var(--border); border-radius: 6px; padding: 5px 12px; }
  .primary.progress {
    color: var(--on-accent); opacity: 1; cursor: progress;
    background: linear-gradient(90deg, var(--accent) var(--p), color-mix(in srgb, var(--accent) 45%, var(--surface)) var(--p));
  }
  .primary { background: var(--accent); color: var(--on-accent); border: 0; border-radius: 8px; padding: 9px 28px; font-weight: 700; }
  .error { background: var(--danger); padding: 8px 12px; border-radius: 6px; }

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
  .slot:focus-visible { outline: 2px solid var(--accent); outline-offset: 8px; border-radius: 12px; }

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
  .caption p { margin: 4px 0; color: var(--muted); font-size: 17px; }
  .system-name { text-transform: uppercase; letter-spacing: 2px; font-size: 13px; color: var(--muted); margin-bottom: 6px !important; }
  .system-logo { color: var(--text); margin-bottom: 8px !important; line-height: 1; }
  .actions { display: flex; gap: 10px; justify-content: center; margin-top: 14px; flex-wrap: wrap; }
  .status { color: var(--ok); }
  .title { display: inline-flex; align-items: center; gap: 8px; }
  .icon { background: none; border: 0; color: var(--muted); font-size: 18px; padding: 2px 4px; border-radius: 4px; }
  .icon:hover, .icon:focus-visible { color: var(--accent); }
  .rename {
    font: 600 26px/1.2 "Atkinson Hyperlegible Next", system-ui, sans-serif; text-align: center; color: inherit;
    background: var(--surface); border: 1px solid var(--accent); border-radius: 6px; padding: 2px 10px; width: min(520px, 90%);
  }
  .original { font-size: 14px; }
  .actions.small { margin-top: 6px; gap: 16px; }
  .link { background: none; border: 0; color: var(--muted); font-size: 15px; text-decoration: underline; text-underline-offset: 3px; }
  .link:hover, .link:focus-visible { color: var(--accent); }
  .rom { font-size: 15px; color: var(--ok) !important; }
  .rom.missing { color: var(--accent) !important; }

  footer { display: flex; gap: 26px; justify-content: center; color: var(--muted); font-size: 15px; padding: 10px 0 6px; flex-wrap: wrap; }
  footer span { display: inline-flex; align-items: center; gap: 4px; }
  kbd.pad { border-radius: 10px; min-width: 22px; font-weight: 700; }
  /* Using the controller: no mouse pointer until the mouse moves again. */
  :global(body:has(main.pad-mode)), :global(body:has(main.pad-mode) *) { cursor: none !important; }
  kbd {
    display: inline-flex; align-items: center; justify-content: center;
    min-width: 24px; height: 24px; padding: 0 6px; box-sizing: border-box; background: var(--surface);
    border: 1px solid var(--border); border-radius: 5px; font: 600 13px/1 "Atkinson Hyperlegible Next", system-ui, sans-serif; color: var(--text);
  }

  aside {
    position: fixed; top: 0; right: 0; bottom: 0; width: min(440px, 100vw);
    background: var(--panel); border-left: 1px solid var(--border); padding: 24px; overflow-y: auto; box-sizing: border-box;
    box-shadow: -12px 0 32px rgba(0,0,0,0.5);
  }
  .close { position: absolute; top: 12px; right: 12px; font-size: 18px; }
  aside h2 { margin: 0 0 14px; }
  .tabs { display: flex; gap: 4px; flex-wrap: wrap; margin-bottom: 8px; }
  .tabs button { background: var(--surface); border: 0; border-radius: 6px; padding: 4px 10px; }
  .tabs button.on { background: var(--accent); color: var(--on-accent); }
  .options { display: grid; gap: 6px; }
  .option { display: flex; justify-content: space-between; align-items: center; gap: 12px; padding: 4px 0; border-bottom: 1px solid var(--surface); }
  .option input[type="text"], .option input[type="number"] {
    width: 150px; background: var(--bg); color: inherit; border: 1px solid var(--border); border-radius: 4px; padding: 3px 6px;
  }
  code { font-size: 13px; color: var(--muted); }
  .muted { color: var(--muted); }
  /* Header controls share one size, border and radius. */
  .tool {
    box-sizing: border-box; height: 38px; padding: 0 16px; display: inline-flex; align-items: center; justify-content: center;
    font: 600 14px/1 "Atkinson Hyperlegible Next", system-ui, sans-serif; color: var(--text);
    background: var(--surface); border: 1px solid var(--border); border-radius: 8px;
    transition: background 0.15s ease, border-color 0.15s ease;
  }
  .tool:hover, .tool:focus-within, .tool:focus-visible { background: var(--surface-2); border-color: color-mix(in srgb, var(--text) 35%, var(--border)); }
  .tool.square { width: 38px; padding: 0; font-size: 17px; }
  .gear { font-size: 19px; }
  .dialog.options { width: min(720px, 94vw); max-height: 88vh; overflow-y: auto; text-align: left; position: relative; padding: 24px 28px; }
  .dialog.options h2 { text-align: left; margin-bottom: 8px; }
  .dialog.options .close { position: absolute; top: 14px; right: 14px; }
  .dialog.options section { border-top: 1px solid var(--border); padding-top: 14px; margin-top: 16px; }
  .dialog.options h3 { margin: 0 0 12px; font-size: 13px; letter-spacing: 1.5px; text-transform: uppercase; color: var(--muted); }
  .dialog.options .row { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 14px; }
  .dialog.options small { color: var(--muted); font-size: 13px; }
  .segmented { display: flex; border: 1px solid var(--border); border-radius: 8px; overflow: hidden; height: 38px; }
  .segmented button { flex: 1; border: 0; background: var(--surface); font-weight: 600; font-size: 14px; }
  .segmented button + button { border-left: 1px solid var(--border); }
  .segmented button.on { background: var(--accent); color: var(--on-accent); }
  .folder { display: flex; gap: 14px; align-items: center; justify-content: space-between; flex-wrap: wrap; }
  .folder .label { display: block; font-size: 14px; color: var(--muted); }
  .folder code { font-size: 14px; color: var(--text); word-break: break-all; }
  .folder p { margin: 0; flex: 1; min-width: 220px; }
  .systems-roms { list-style: none; padding: 0; margin: 14px 0 0; display: grid; gap: 6px; }
  .systems-roms li { display: grid; grid-template-columns: 170px 1fr auto; align-items: center; gap: 12px; padding: 8px 12px; border-radius: 8px; background: var(--surface); }
  .systems-roms .status { font-size: 14px; color: var(--muted); }
  .systems-roms .status.done { color: var(--ok); }
  .systems-roms .status.warn { color: var(--warn); }
  .tool.small { height: 32px; font-size: 13px; padding: 0 12px; }
  .rom.note { color: var(--muted) !important; }
  .primary:disabled { opacity: 0.55; cursor: default; }
  .dialog.add-port { width: min(560px, 92vw); text-align: left; }
  .dialog.add-port h2 { text-align: center; }
  .dialog.add-port .file { font-size: 13px; word-break: break-all; margin: 0 0 14px; }
  .field { display: flex; flex-direction: column; gap: 4px; margin-bottom: 12px; }
  .field span { font-size: 14px; color: var(--muted); }
  .field select { appearance: none; -webkit-appearance: none; padding-right: 34px; cursor: pointer;
    background-image: linear-gradient(45deg, transparent 50%, currentColor 50%), linear-gradient(135deg, currentColor 50%, transparent 50%);
    background-position: calc(100% - 18px) 55%, calc(100% - 13px) 55%; background-size: 5px 5px; background-repeat: no-repeat; }
  .field select option { background: var(--panel); color: var(--text); }
  .field select, .field input {
    font: inherit; color: var(--text); background-color: var(--surface); border: 1px solid var(--border);
    border-radius: 8px; padding: 8px 10px;
  }
  .warn-text { color: var(--warn); font-size: 14px; margin: 0 0 12px; }
  .choices button:disabled { opacity: 0.5; cursor: default; }
  .scrim.center { place-items: center; padding-top: 0; }
  .dialog {
    width: min(380px, 90vw); background: var(--panel); border: 1px solid var(--border); border-radius: 12px;
    box-shadow: 0 24px 60px rgba(0,0,0,0.6); padding: 22px 22px 14px; text-align: center;
  }
  .dialog h2 { margin: 0 0 18px; font-size: 20px; }
  .choices { display: flex; gap: 12px; justify-content: center; }
  .choices button {
    min-width: 110px; padding: 9px 0; border-radius: 8px; border: 1px solid var(--border);
    background: var(--surface-2); font-weight: 600;
  }
  .choices button.on { background: var(--accent); border-color: var(--accent); color: var(--on-accent); }
  .dialog-hints { display: flex; gap: 16px; justify-content: center; margin: 16px 0 0; font-size: 14px; color: var(--muted); }
  .dialog-hints span { display: inline-flex; align-items: center; gap: 4px; }
  .scrim { position: fixed; inset: 0; background: var(--scrim); display: grid; place-items: start center; padding-top: 12vh; z-index: 200; }
  .search { width: min(640px, 92vw); background: var(--panel); border: 1px solid var(--border); border-radius: 12px; box-shadow: 0 24px 60px rgba(0,0,0,0.6); overflow: hidden; }
  .query { width: 100%; box-sizing: border-box; border: 0; border-bottom: 1px solid var(--border); background: transparent; color: inherit; font: 18px "Atkinson Hyperlegible Next", system-ui, sans-serif; padding: 16px 18px; outline: none; }
  .results { list-style: none; margin: 0; padding: 6px; max-height: 56vh; overflow-y: auto; }
  .results button { width: 100%; display: flex; gap: 12px; align-items: center; background: none; border: 0; border-radius: 8px; padding: 6px 8px; text-align: left; }
  .results button.on { background: var(--surface-2); }
  .results .thumb { width: 56px; height: 40px; flex: none; border-radius: 4px; overflow: hidden; background: var(--surface); }
  .results .thumb img { width: 100%; height: 100%; object-fit: cover; }
  .results .name { display: block; font-weight: 600; }
  .results .meta { display: block; font-size: 14px; color: var(--muted); }
  .results .installed { color: var(--ok); }
  .results .none { padding: 14px; color: var(--muted); }
  .link.inline { font-size: inherit; padding: 0; color: var(--text); }
  .known { list-style: none; padding: 0; margin: 12px 0 0; display: grid; gap: 10px; }
  .known li { display: flex; gap: 12px; align-items: center; padding: 8px; border-radius: 8px; background: var(--surface); }
  .thumb { width: 64px; height: 46px; flex: none; padding: 0; border: 0; border-radius: 4px; overflow: hidden; background: var(--surface); }
  .thumb img { width: 100%; height: 100%; object-fit: cover; }
  .info { display: flex; flex-direction: column; align-items: flex-start; gap: 2px; min-width: 0; }
  .info .name { background: none; border: 0; padding: 0; font-weight: 600; text-align: left; }
  .info .name:hover, .info .name:focus-visible { color: var(--accent); }
  .info .meta { font-size: 14px; color: var(--muted); }
  .info .installed { color: var(--ok); }
  .info .link { padding: 0; font-size: 14px; }
</style>
