# portshelf

A shelf for the native PC ports of console games (N64: Recompiled ports, Harbour Masters ports, decompilations). Each system is a console you open; inside, every port is a cartridge or a disc you can start, configure and look up.

**Work in progress.**

## What it does

- Two-level carousel: systems, then the ports on that system (keyboard, mouse or gamepad)
- Starts installed ports straight into the game (skipping each port's own launcher), detached from the shelf
- Asks for the game file first, and sets it up the way each port expects (recomp ports get a big-endian copy, Dusklight gets the path, Harbour Masters ports get the ROM next to them)
- Reads and edits the ports' own settings files (RecompFrontend JSON for now)
- Catalog of known ports in `catalog/ports.json`; ports not installed can be shown with a link to their project

The library (what is installed and how to start it) lives in `~/.config/portshelf/library.json` and is created by scanning the usual install locations on first run. Cover art goes in `~/.config/portshelf/covers/<port id>.png`.

## Controls

| Action | Keyboard | Gamepad |
|---|---|---|
| Browse | Left / Right | D-pad, left stick |
| Open system / play | Enter | A |
| Back | Esc | B |
| Settings | S | Y |

## Development

Needs Rust, Node and pnpm, plus WebKitGTK 4.1 on Linux.

```bash
pnpm install
pnpm app   # tauri dev, also rebuilding when catalog/ changes; sets WEBKIT_DISABLE_DMABUF_RENDERER=1 (Wayland + NVIDIA)
```

On Linux the window has no title bar (tiling compositors do not draw one, so GTK would add its own buttons); Windows and macOS keep their native one.

## Credits

Console photos: Evan-Amos and others on Wikimedia Commons, see [CREDITS.md](CREDITS.md).


Built with [Tauri](https://tauri.app) and [Svelte](https://svelte.dev). The port list started from [awesome-unofficial-pc-ports](https://github.com/Sebastrion/awesome-unofficial-pc-ports).
