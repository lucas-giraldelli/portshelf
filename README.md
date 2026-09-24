<p align="center"><img src="docs/icon.svg" width="128" alt="PortShelf icon"></p>

<h1 align="center">PortShelf</h1>

PortShelf is a launcher for native PC ports of console games: static recompilations (N64: Recompiled and others), decompilation projects (Harbour Masters ports, Dusklight) and builders that produce a port from your own game file. Each system appears as its console; opening it shows its ports as cartridges or discs that can be started, configured and looked up.

**Work in progress.**

![Browsing systems and games in PortShelf](docs/portshelf.webp)

## Features

- Two-level carousel: systems ordered by release year and wrapping around, then the ports on each system
- Keyboard, mouse and controller navigation across the whole app; the key hints at the bottom follow the last input device used, and the pointer hides while a controller is in use
- Installs ports from their projects' latest GitHub release (zip, tar.gz or AppImage, nested archives included) into `~/.local/share/PortShelf/ports/<port id>/`; only the port is downloaded, the game file always comes from the user
- Ports start straight into the game, skipping each port's own launcher; the shelf hides while a game runs and returns when it exits
- A game file is required before a port can start; it is set up the way each port expects (a big-endian copy for recompilation ports, the file path for Dusklight, a link next to the executable for Harbour Masters ports)
- ROM folder: chosen once, it gets a folder per game (`<system>/<game>/`, for example `n64/chameleon_twist/`). A file placed in a game's folder is used for that game; files anywhere else in the ROM folder are identified by the game code stored in them (N64 cartridge header; GameCube disc header, also inside RVZ and WIA images) or by their No-Intro or Redump name, and copies that look modified are skipped, since ports need the original game. The folder is synced on start and on Rescan; files for ports not installed yet are set up when the port is installed
- Settings editor for ports that store their options as JSON (the RecompFrontend family)
- Cover art found automatically, with manual cover choice, renaming and online search per game
- Search across every system
- A list of every known port for a system, with links to each project
- Cartridge and disc templates per system, with the cover shown whole inside the label area and per-game cartridge colours

## Controls

| Action | Keyboard | Controller |
|---|---|---|
| Browse | Left / Right | D-pad or left stick |
| Open system / play | Enter | South face button (Xbox A, PlayStation Cross) |
| Back | Esc | East face button (Xbox B, PlayStation Circle) |
| Quit (from the systems screen, then confirm) | Esc | East face button |
| Settings | S | North face button (Xbox Y, PlayStation Triangle) |
| Known ports of the system | K | West face button (Xbox X, PlayStation Square) |
| Rename | R | |
| Open the ROM folder | I | |
| Search every system | Ctrl+F | Start |
| Every known port or installed only | Tab | Select |
| Previous or next system while viewing games | | LB / RB |

Controllers are read natively with gilrs, so they work where the webview has no Gamepad API. Only the focused PortShelf window reacts to the controller.

## Data

| What | Location on Linux |
|---|---|
| Ports installed by PortShelf | `~/.local/share/PortShelf/ports/<port id>/` (Windows: `%LOCALAPPDATA%\PortShelf\ports`, macOS: `~/Library/Application Support/PortShelf/ports`) |
| Library (installed ports and how to start them) | `~/.config/portshelf/library.json` |
| Cover art | `~/.config/portshelf/covers/<port id>.png` (replaced covers are kept in `covers/replaced/`) |
| Cover index cache | `~/.cache/portshelf/` |
| Game files | the ROM folder you choose, organised as `<system>/<game>/` |

The library is created on first run by scanning the usual install locations; Rescan refreshes the entries it finds and keeps covers chosen by hand.

Cover art comes from [libretro-thumbnails](https://github.com/libretro-thumbnails), matched by the No-Intro or Redump title recorded in the catalog, preferring the USA release. A cover chosen by hand is never replaced by scraping.

## Catalog

`catalog/ports.json` lists the known ports: system, kind (recompilation, decompilation or builder), project page, No-Intro or Redump title, an optional cartridge colour, and an optional `install` section. The install section names, per operating system, the words that identify the release asset and the program to start, plus launch arguments, the settings folder and how the port expects its game file. `cargo run --example install -- <port id> <folder>` (in `src-tauri`) tests a rule without the interface, and `cargo run --example scan -- <folder>` shows which ports the game files in a folder match. Entries can also carry `codes` (N64 game codes such as `NDOE`, GameCube IDs such as `GZ2E`) and `authors`. Each system entry has its name, release year, media type and accent colour.

The list was put together from [PCGamingWiki's list of unofficial ports](https://www.pcgamingwiki.com/wiki/List_of_unofficial_ports) and [awesome-unofficial-pc-ports](https://github.com/Sebastrion/awesome-unofficial-pc-ports).

## Development

Requirements: Rust, Node.js, pnpm and, on Linux, WebKitGTK 4.1.

```bash
pnpm install
pnpm app      # development build; also rebuilds when catalog/ changes
pnpm bundle   # AppImage in src-tauri/target/release/bundle/appimage/
```

On Linux the app sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` for itself, which avoids a Wayland protocol error with NVIDIA drivers. `pnpm bundle` sets `NO_STRIP`, needed on distributions whose libraries are newer than the `strip` bundled with linuxdeploy. The development window is titled `PortShelf (dev)`, so a window manager rule can tell it apart from the installed app.

On Linux the window has no title bar, since tiling compositors do not draw one and GTK would add its own buttons; Windows and macOS keep their native title bar.

## Credits

Built with [Tauri](https://tauri.app) and [Svelte](https://svelte.dev). Console photographs by Evan-Amos and others on Wikimedia Commons; see [CREDITS.md](CREDITS.md) for every image source and license.
