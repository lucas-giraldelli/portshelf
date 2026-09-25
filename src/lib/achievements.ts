// PortShelf's own achievements, as the backend reports them.

import { prefs } from "$lib/prefs.svelte";

/** Progress of one port's set. */
export type Summary = { port: string; unlocked: number; total: number; points: number; total_points: number; last: number | null };

/** One achievement with its unlock time (Unix seconds) when unlocked. */
export type AchievementStatus = {
  id: string;
  points: number;
  title: Record<string, string>;
  description: Record<string, string>;
  unlocked: number | null;
};

/** An achievement's text in the shelf's language, English otherwise. */
export const localized = (texts: Record<string, string>) => texts[prefs.lang] ?? texts.en ?? "";
