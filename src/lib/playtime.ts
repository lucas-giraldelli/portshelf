// Time played, as the caption shows it: "12 h 30 min played · yesterday".

import { tr } from "$lib/prefs.svelte";

export type Playtime = { total_secs: number; last_played: number; sessions: number };

export function formatDuration(secs: number): string {
  const minutes = Math.floor(secs / 60);
  if (minutes < 60) return tr("play.minutes", { m: Math.max(1, minutes) });
  return tr("play.hours", { h: Math.floor(minutes / 60), m: minutes % 60 });
}

export function formatLastPlayed(unix: number): string {
  const day = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  const days = Math.round((day(new Date()) - day(new Date(unix * 1000))) / 86400000);
  if (days <= 0) return tr("play.today");
  if (days === 1) return tr("play.yesterday");
  if (days < 30) return tr("play.daysAgo", { n: days });
  return new Date(unix * 1000).toLocaleDateString(tr("play.locale"), { day: "numeric", month: "short", year: "numeric" });
}
