// Shelf state and the actions on it: the catalog, the user's library, covers, game files,
// installs and launching. Components read it and call its methods; navigation (which system
// and game are in view, which dialog is open) lives in the page.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { tr } from "$lib/prefs.svelte";
import type { Key } from "$lib/i18n";
import type { Catalog, Library, Port, RomStatus } from "$lib/types";

export type InstallProgress = { stage: string; done: number; total: number | null };
export type RomSummary = { ready: number; installed: number; assigned: number };

const MEDIA = ["n64-cartridge", "snes-cartridge", "gba-cartridge", "md-cartridge", "gc-disc", "ps1-disc", "ps2-disc", "x360-disc"];
const CONSOLES = ["n64", "gc", "snes", "gba", "ps1", "ps2", "md", "x360"];

// Images are decoded before they are shown; otherwise WebKit decodes each one the first time
// it comes into the carousel and the item flickers.
function decode(src: string) {
  const img = new Image();
  img.src = src;
  return img.decode().catch(() => {});
}

class Shelf {
  catalog = $state<Catalog | null>(null);
  library = $state<Library | null>(null);
  covers = $state<Record<string, string | null>>({});
  /** Whether each installed port has its game file set up. */
  roms = $state<Record<string, RomStatus>>({});
  installing = $state<Record<string, InstallProgress>>({});
  romSummary = $state<RomSummary | null>(null);
  syncing = $state(false);
  os = $state("linux");
  error = $state("");
  status = $state("");

  constructor() {
    invoke<string>("platform").then((p) => (this.os = p));
    listen<InstallProgress & { id: string }>("install-progress", (e) => {
      this.installing[e.payload.id] = e.payload;
    });
    for (const media of MEDIA) decode(`/media/${media}.png`);
    for (const id of CONSOLES) decode(`/consoles/${id}.png`);
  }

  // ---- queries ----

  isInstalled = (id: string) => !!this.library?.installed[id];
  displayName = (port: Port) => this.library?.overrides?.[port.id]?.name ?? port.name;
  romReady = (id: string) => this.roms[id]?.ready ?? false;
  /** Ports that ask for their game file themselves (PortShelf does not know where they keep it). */
  selfManaged = (id: string) => this.isInstalled(id) && !this.library?.installed[id]?.rom;
  canInstall = (port: Port) => !!port.install?.[this.os as "linux" | "windows" | "macos"];
  /** Not made for this operating system (for example, Windows only). */
  unavailable = (port: Port) => !!port.platforms && !port.platforms.includes(this.os);
  platformLabel = (port: Port) =>
    tr("game.onlyOn", {
      platforms: (port.platforms ?? []).map((p) => ({ windows: "Windows", linux: "Linux", macos: "macOS" })[p] ?? p).join(` ${tr("game.and")} `),
    });
  kindLabel = (kind: string) => tr(`kind.${kind}` as Key);
  /** Catalog authors, or the GitHub owner of the project. */
  authorsOf = (port: Port) => port.authors ?? port.repo.match(/^https:\/\/github\.com\/([^/]+)/)?.[1] ?? "";
  portById = (id: string) => this.catalog?.ports.find((p) => p.id === id);
  /** Every catalog port of a system, installed ones first. */
  knownPorts = (consoleId: string) =>
    (this.catalog?.ports ?? [])
      .filter((p) => p.console === consoleId)
      .sort((a, b) => Number(this.isInstalled(b.id)) - Number(this.isInstalled(a.id)) || this.displayName(a).localeCompare(this.displayName(b)));
  /** Installed ports of a system that have their game file, out of those installed. */
  systemRomStatus = (consoleId: string) => {
    const installed = (this.catalog?.ports ?? []).filter((p) => p.console === consoleId && this.isInstalled(p.id));
    return { ready: installed.filter((p) => this.romReady(p.id)).length, installed: installed.length };
  };
  /** Consoles in release order. */
  get consoles() {
    return Object.entries(this.catalog?.consoles ?? {}).sort((a, b) => a[1].year - b[1].year);
  }

  // ---- messages ----

  flash(message: string) {
    this.status = message;
    setTimeout(() => (this.status = ""), 3000);
  }
  fail(e: unknown) {
    this.error = String(e);
  }

  // ---- loading ----

  async load() {
    try {
      this.catalog = await invoke<Catalog>("get_catalog");
      this.library = await invoke<Library>("get_library");
      await Promise.all(this.catalog.ports.map((port) => this.loadCover(port.id)));
      this.scrapeMissing();
      await this.syncRoms();
      await this.refreshInstalledRoms();
    } catch (e) {
      this.fail(e);
    }
  }

  private lastRefresh = 0;
  /** New ports and game files are picked up on start and whenever the window regains focus. */
  async refreshAll() {
    if (Date.now() - this.lastRefresh < 3000 || !this.catalog) return;
    this.lastRefresh = Date.now();
    this.library = await invoke("rescan");
    await this.syncRoms();
    await this.refreshInstalledRoms();
  }

  async refreshRom(port: Port) {
    if (!this.isInstalled(port.id)) return;
    try {
      this.roms[port.id] = await invoke<RomStatus>("rom_status", { id: port.id, console: port.console });
    } catch (e) {
      this.fail(e);
    }
  }
  refreshInstalledRoms() {
    return Promise.all((this.catalog?.ports ?? []).filter((p) => this.isInstalled(p.id)).map((p) => this.refreshRom(p)));
  }

  // ---- covers ----

  async loadCover(id: string) {
    const cover = await invoke<string | null>("get_cover", { id });
    if (cover) await decode(cover);
    this.covers[id] = cover;
  }

  /** Fill in missing covers one at a time in the background (libretro-thumbnails). */
  async scrapeMissing() {
    for (const port of this.catalog?.ports ?? []) {
      if (this.covers[port.id]) continue;
      try {
        const found = await invoke<string | null>("scrape_cover", { id: port.id, console: port.console, title: port.title ?? port.name, force: false });
        if (found) await this.loadCover(port.id);
      } catch {
        // No box art for this one; it keeps its drawn label.
      }
    }
  }

  async findCover(port: Port, title = this.displayName(port)) {
    try {
      await invoke("unlock_cover", { id: port.id });
      const found = await invoke<string | null>("scrape_cover", { id: port.id, console: port.console, title, force: true });
      await this.loadCover(port.id);
      this.flash(found ? tr("msg.cover", { file: found.replace(/\.png$/, "") }) : tr("msg.noCover"));
    } catch (e) {
      this.fail(e);
    }
  }

  async pickCover(port: Port) {
    const picked = await open({ title: tr("pick.cover", { name: this.displayName(port) }), filters: [{ name: tr("pick.images"), extensions: ["png", "jpg", "jpeg", "webp"] }] });
    if (typeof picked !== "string") return;
    try {
      await invoke("set_cover", { id: port.id, path: picked });
      await this.loadCover(port.id);
      this.library = await invoke("get_library");
    } catch (e) {
      this.fail(e);
    }
  }

  async rename(port: Port, draft: string) {
    const custom = draft.trim() === port.name ? null : draft.trim();
    try {
      this.library = await invoke("rename", { id: port.id, name: custom });
      // A new name is also a better search term, unless the cover was picked by hand.
      if (custom && !this.library?.overrides?.[port.id]?.cover_locked) await this.findCover(port, custom);
    } catch (e) {
      this.fail(e);
    }
  }

  // ---- game files ----

  async syncRoms() {
    if (!this.library?.roms_dir) return;
    this.syncing = true;
    try {
      this.romSummary = await invoke<RomSummary>("sync_roms");
      this.library = await invoke("get_library");
      if (this.romSummary.assigned) this.flash(tr("msg.filesFound", { count: this.romSummary.assigned }));
    } catch (e) {
      this.fail(e);
    } finally {
      this.syncing = false;
    }
  }

  async chooseRomFolder() {
    const dir = await open({ directory: true, title: tr("pick.romFolder"), defaultPath: this.library?.roms_dir || undefined });
    if (typeof dir !== "string") return;
    this.syncing = true;
    try {
      this.romSummary = await invoke<RomSummary>("set_roms_dir", { dir });
      this.library = await invoke("get_library");
      await this.refreshInstalledRoms();
      this.flash(tr("msg.romFolderReady"));
    } catch (e) {
      this.fail(e);
    } finally {
      this.syncing = false;
    }
  }

  /** File chooser in a system's folder; the file goes to the game whose folder it is in. */
  async chooseSystemFile(consoleId?: string) {
    if (!this.library?.roms_dir) return this.chooseRomFolder();
    const picked = await open({
      title: consoleId ? tr("pick.systemFiles", { system: this.catalog?.consoles[consoleId].name ?? "" }) : tr("pick.gameFiles"),
      defaultPath: consoleId ? `${this.library.roms_dir}/${consoleId}` : this.library.roms_dir,
    });
    if (typeof picked !== "string") return;
    try {
      const ids = await invoke<string[]>("assign_rom", { path: picked });
      this.library = await invoke("get_library");
      const ports = ids.map((id) => this.portById(id)).filter((p): p is Port => !!p);
      await Promise.all(ports.filter((p) => this.isInstalled(p.id)).map((p) => this.refreshRom(p)));
      this.flash(tr("msg.fileSetFor", { names: ports.map(this.displayName).join(` ${tr("game.and")} `) }));
    } catch (e) {
      this.fail(e);
    }
  }

  /** File chooser for one port's game file. */
  async selectRom(port: Port) {
    const disc = this.catalog?.consoles[port.console].media === "disc";
    const picked = await open({
      title: tr("pick.gameFile", { system: this.catalog?.consoles[port.console].name ?? "", name: this.displayName(port) }),
      defaultPath: this.roms[port.id]?.browse_dir,
      filters: [{ name: tr("pick.gameFiles"), extensions: disc ? ["iso", "rvz", "gcm", "ciso", "wbfs", "nkit.iso"] : ["z64", "n64", "v64"] }],
    });
    if (typeof picked !== "string") return;
    try {
      await invoke("select_rom", { id: port.id, path: picked });
      await this.refreshRom(port);
      this.flash(tr("msg.fileReady"));
    } catch (e) {
      this.fail(e);
    }
  }

  // ---- installing and launching ----

  installLabel(id: string) {
    const p = this.installing[id];
    if (!p) return "";
    const mb = (n: number) => (n / 1048576).toFixed(0);
    if (p.stage === "downloading") return p.total ? tr("game.downloading", { done: mb(p.done), total: mb(p.total) }) : tr("game.downloadingNoTotal", { done: mb(p.done) });
    return p.stage === "unpacking" ? tr("game.unpacking") : tr("game.finishing");
  }

  /** Returns true when the port got installed. */
  async installPort(port: Port) {
    if (this.installing[port.id]) return false;
    this.installing[port.id] = { stage: "downloading", done: 0, total: null };
    try {
      this.library = await invoke("install_port", { id: port.id });
      delete this.installing[port.id];
      await this.refreshRom(port);
      this.flash(tr(this.romReady(port.id) ? "msg.installed" : "msg.installedNeedsFile", { name: this.displayName(port) }));
      return true;
    } catch (e) {
      delete this.installing[port.id];
      this.fail(e);
      return false;
    }
  }

  /** Adds a program installed outside PortShelf, as a catalog port or a new one. Returns its id. */
  async addInstall(exec: string, choice: string, name: string, consoleId: string) {
    try {
      const id = choice === "new" ? await invoke<string>("add_custom_port", { name, console: consoleId }) : choice;
      this.library = await invoke("add_install", { id, exec });
      this.catalog = await invoke<Catalog>("get_catalog");
      const port = this.portById(id);
      if (port) {
        await this.refreshRom(port);
        this.flash(tr("msg.added", { name: this.displayName(port) }));
      }
      return id;
    } catch (e) {
      this.fail(e);
      return null;
    }
  }

  async play(port: Port) {
    try {
      await invoke("launch", { id: port.id });
      this.flash(tr("msg.started", { name: this.displayName(port) }));
    } catch (e) {
      this.fail(e);
    }
  }
}

export const shelf = new Shelf();

/** Which catalog port a program most likely is, from its file and folder names ("new" if none). */
export function guessPort(exec: string): string {
  const squash = (t: string) => t.toLowerCase().replace(/[^a-z0-9]/g, "");
  const parts = exec.split("/");
  const file = squash((parts.pop() ?? "").replace(/\.(appimage|exe|x86_64)$/i, ""));
  const folder = squash(parts.pop() ?? "");
  let best = { id: "new", score: 0 };
  for (const port of shelf.catalog?.ports ?? []) {
    const words = [port.id, port.name, port.title ?? "", port.repo.split("/").filter(Boolean).pop() ?? "", ...port.id.split("-")]
      .map(squash)
      .filter((w) => w.length >= 3);
    for (const hay of [file, folder]) {
      if (!hay) continue;
      for (const w of words) {
        const score = hay.includes(w) ? w.length : w.includes(hay) && hay.length >= 4 ? hay.length : 0;
        const bonus = shelf.isInstalled(port.id) ? 0 : 0.5;
        if (score && score + bonus > best.score) best = { id: port.id, score: score + bonus };
      }
    }
  }
  return best.id;
}
