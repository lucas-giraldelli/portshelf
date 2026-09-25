// Fullscreen that survives games. The shelf hides while a game runs, and on Wayland a hidden
// window comes back as a new one, without its fullscreen state; the preference follows the
// window (F11, the button, the settings, or the compositor's own shortcut) and is applied again
// on start and whenever a game closes. Windows and macOS keep the state; reapplying is harmless.

import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { prefs } from "$lib/prefs.svelte";

const win = getCurrentWindow();
const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

/** A game is running: the window is hidden, and its size says nothing about the preference. */
let away = false;
/** A request is being applied: the sizes seen meanwhile are not the user's choice. */
let applying = 0;

/** Asks for the state until the window has it: compositors ignore requests made while the
 *  window is still appearing (on start, or right after a game closes). */
async function apply(on: boolean) {
  const run = ++applying;
  try {
    for (let i = 0; i < 15 && run === applying; i++) {
      if ((await win.isFullscreen()) === on) return;
      await win.setFullscreen(on).catch(() => {});
      await sleep(200);
    }
  } finally {
    if (run === applying) applying = 0;
  }
}

export function setFullscreen(on: boolean) {
  prefs.fullscreen = on;
  apply(on);
}

/** Toggles from what the window actually is, not from the saved preference. */
export async function toggleFullscreen() {
  setFullscreen(!(await win.isFullscreen()));
}

/** Call right before a game starts. */
export function leaveForGame() {
  away = true;
}

/** Call when a game could not start: the window stayed. */
export function cancelLeave() {
  away = false;
}

export function startFullscreenSync() {
  if (prefs.fullscreen) apply(true);
  win.onResized(async () => {
    if (away || applying) return;
    prefs.fullscreen = await win.isFullscreen();
  });
  listen("game-exited", async () => {
    if (prefs.fullscreen) await apply(true);
    // The window's first sizes after coming back are not the user's choice.
    setTimeout(() => (away = false), 600);
  });
}
