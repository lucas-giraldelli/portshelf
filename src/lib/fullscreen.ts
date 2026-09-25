// Fullscreen that survives games. The shelf hides while a game runs, and on Wayland a hidden
// window comes back as a new one, without its fullscreen state; the preference follows the
// window (F11, the settings, or the compositor's own shortcut) and is applied again on start
// and whenever a game closes. Windows and macOS keep the state, and reapplying it is harmless.

import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { prefs } from "$lib/prefs.svelte";

const win = getCurrentWindow();
/** A game is running: the window is hidden, and its size says nothing about the preference. */
let away = false;

export function setFullscreen(on: boolean) {
  prefs.fullscreen = on;
  win.setFullscreen(on).catch(() => {});
}

export const toggleFullscreen = () => setFullscreen(!prefs.fullscreen);

/** Call right before a game starts. */
export function leaveForGame() {
  away = true;
}

/** Call when a game could not start: the window stayed. */
export function cancelLeave() {
  away = false;
}

export function startFullscreenSync() {
  if (prefs.fullscreen) win.setFullscreen(true).catch(() => {});
  win.onResized(async () => {
    if (!away) prefs.fullscreen = await win.isFullscreen();
  });
  listen("game-exited", async () => {
    if (prefs.fullscreen) await win.setFullscreen(true).catch(() => {});
    // The window's first sizes after coming back are not the user's choice.
    setTimeout(() => (away = false), 600);
  });
}
