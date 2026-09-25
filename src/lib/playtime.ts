// Time played, as the caption shows it: "9.8 hours", last played "yesterday".

import { tr } from "$lib/prefs.svelte";

export type Playtime = { total_secs: number; last_played: number; sessions: number };

/** Hours with one decimal, like Steam ("9.8 hours"); minutes under an hour. */
export function formatHours(secs: number): string {
  const minutes = Math.floor(secs / 60);
  if (minutes < 1) return tr("play.underMinute");
  if (minutes < 2) return tr("play.minute");
  if (minutes < 60) return tr("play.minutesLong", { m: minutes });
  const hours = (secs / 3600).toLocaleString(tr("play.locale"), { maximumFractionDigits: 1 });
  return tr(hours === "1" ? "play.hour" : "play.hoursLong", { h: hours });
}

export function formatLastPlayed(unix: number): string {
  const day = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  const days = Math.round((day(new Date()) - day(new Date(unix * 1000))) / 86400000);
  if (days <= 0) return tr("play.today");
  if (days === 1) return tr("play.yesterday");
  if (days < 30) return tr("play.daysAgo", { n: days });
  return new Date(unix * 1000).toLocaleDateString(tr("play.locale"), { day: "numeric", month: "short", year: "numeric" });
}
