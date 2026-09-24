// User preferences kept on this machine: theme, light / dark mode and language, plus the
// translate function every component uses for its text.

import { applyTheme, themes, type Mode } from "$lib/themes";
import { defaultLang, translate, type Key, type Lang } from "$lib/i18n";

const stored = (key: string) => {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
};

export const prefs = $state({
  theme: stored("portshelf.theme") ?? "arcade",
  mode: ((stored("portshelf.mode") as Mode) ?? "dark") as Mode,
  lang: ((stored("portshelf.lang") as Lang) ?? defaultLang()) as Lang,
});

/** Interface text in the chosen language. */
export const tr = (key: Key, vars: Record<string, string | number> = {}) => translate(prefs.lang, key, vars);

export function nextTheme(): string {
  const ids = Object.keys(themes);
  prefs.theme = ids[(ids.indexOf(prefs.theme) + 1) % ids.length];
  return themes[prefs.theme].name;
}

// Apply and remember the preferences whenever they change.
$effect.root(() => {
  $effect(() => {
    applyTheme(prefs.theme, prefs.mode);
    try {
      localStorage.setItem("portshelf.theme", prefs.theme);
      localStorage.setItem("portshelf.mode", prefs.mode);
      localStorage.setItem("portshelf.lang", prefs.lang);
    } catch {
      // not persisted; the preferences still apply for this session
    }
  });
});
