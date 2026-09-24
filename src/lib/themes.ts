// Colour themes. Each has a dark and a light variant; the page reads them as CSS variables.

export type Mode = "dark" | "light";

export interface Palette {
  bg: string; // window background
  panel: string; // side panels, dialogs
  surface: string; // list rows, tabs, inputs
  surface2: string; // hovered / selected rows
  border: string;
  text: string;
  muted: string; // secondary text, hints
  accent: string; // primary buttons, highlights
  onAccent: string; // text on accent
  ok: string;
  warn: string;
  danger: string;
  scrim: string; // behind dialogs
}

export interface Theme {
  name: string;
  description: string;
  dark: Palette;
  light: Palette;
}

export const themes: Record<string, Theme> = {
  arcade: {
    name: "Arcade",
    description: "Warm amber on charcoal, like a cabinet in a dark room",
    dark: {
      bg: "#121116", panel: "#1c1b21", surface: "#26252c", surface2: "#322f38", border: "#4d4852",
      text: "#f3eee6", muted: "#c3bab0", accent: "#f5b547", onAccent: "#1d1508",
      ok: "#a6dd8f", warn: "#f5b547", danger: "#ff8a7a", scrim: "rgba(6, 6, 8, 0.65)",
    },
    light: {
      bg: "#f6f1e8", panel: "#ffffff", surface: "#efe7da", surface2: "#e5d9c6", border: "#cdbfa9",
      text: "#221c14", muted: "#5c5145", accent: "#c77d05", onAccent: "#ffffff",
      ok: "#2f7d1f", warn: "#a45d00", danger: "#b3261e", scrim: "rgba(40, 30, 20, 0.35)",
    },
  },
  console: {
    name: "Console",
    description: "Grey plastic and the red, yellow, green and blue of the buttons",
    dark: {
      bg: "#17181b", panel: "#212327", surface: "#2b2e33", surface2: "#373b41", border: "#50555d",
      text: "#eceef1", muted: "#b5bac2", accent: "#e53935", onAccent: "#ffffff",
      ok: "#6fcf6f", warn: "#ffcc33", danger: "#ff7b72", scrim: "rgba(5, 6, 8, 0.65)",
    },
    light: {
      bg: "#dcdde0", panel: "#f1f2f4", surface: "#e6e7ea", surface2: "#d3d5d9", border: "#a9adb4",
      text: "#1b1d21", muted: "#4a4f57", accent: "#c62828", onAccent: "#ffffff",
      ok: "#1f7a2e", warn: "#8a6100", danger: "#b3261e", scrim: "rgba(20, 22, 26, 0.35)",
    },
  },
  phosphor: {
    name: "Phosphor",
    description: "Green CRT glow on black glass",
    dark: {
      bg: "#070b08", panel: "#0e1510", surface: "#152018", surface2: "#1d2c21", border: "#2f4a36",
      text: "#d9fbe0", muted: "#9cc9a6", accent: "#46e27a", onAccent: "#04140a",
      ok: "#46e27a", warn: "#e8d34d", danger: "#ff7a6e", scrim: "rgba(0, 4, 1, 0.7)",
    },
    light: {
      bg: "#eef5ee", panel: "#ffffff", surface: "#e1ece2", surface2: "#cfe0d1", border: "#9dbba3",
      text: "#0f2615", muted: "#3f5c46", accent: "#1c8a45", onAccent: "#ffffff",
      ok: "#1c8a45", warn: "#8a6d00", danger: "#b3261e", scrim: "rgba(10, 30, 15, 0.3)",
    },
  },
  pop: {
    name: "Pop",
    description: "Saturday-morning colour: indigo, hot pink and cyan",
    dark: {
      bg: "#15122b", panel: "#1f1a3d", surface: "#29234f", surface2: "#352d65", border: "#554a8f",
      text: "#f5f2ff", muted: "#c7bfeb", accent: "#ff5fa2", onAccent: "#1a0612",
      ok: "#5ff0c8", warn: "#ffd166", danger: "#ff8080", scrim: "rgba(8, 5, 20, 0.65)",
    },
    light: {
      bg: "#fff6ec", panel: "#ffffff", surface: "#fbe9dc", surface2: "#f6d8c6", border: "#e0b9a3",
      text: "#2a1636", muted: "#5d4468", accent: "#e03a7e", onAccent: "#ffffff",
      ok: "#0f8a6a", warn: "#9a6200", danger: "#b3261e", scrim: "rgba(42, 22, 54, 0.3)",
    },
  },
};

const cssName = (key: string) => "--" + key.replace(/[A-Z]/g, (c) => "-" + c.toLowerCase());

export function applyTheme(id: string, mode: Mode) {
  const palette = (themes[id] ?? themes.arcade)[mode];
  const root = document.documentElement;
  for (const [key, value] of Object.entries(palette)) root.style.setProperty(cssName(key), value);
  root.style.colorScheme = mode;
}
