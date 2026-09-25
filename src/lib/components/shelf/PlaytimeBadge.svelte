<script lang="ts">
  // Time played and the day last played, as two labelled figures under the game's name.
  import { tr } from "$lib/prefs.svelte";
  import { formatHours, formatLastPlayed, type Playtime } from "$lib/playtime";

  let { played }: { played: Playtime } = $props();
</script>

<div class="badges">
  <div class="badge">
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <circle cx="12" cy="12" r="9" />
      <path d="M12 7v5l3 2" />
    </svg>
    <div>
      <span class="label">{tr("play.timeLabel")}</span>
      <span class="value">{formatHours(played.total_secs)}</span>
    </div>
  </div>
  <div class="badge">
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <rect x="4" y="5" width="16" height="15" rx="2" />
      <path d="M4 10h16M9 3v4M15 3v4" />
    </svg>
    <div>
      <span class="label">{tr("play.lastLabel")}</span>
      <span class="value">{formatLastPlayed(played.last_played)}</span>
    </div>
  </div>
</div>

<style>
  .badges { display: flex; justify-content: center; gap: 28px; margin: 10px 0 4px; }
  .badge { display: flex; align-items: center; gap: 10px; text-align: left; }
  svg { width: 30px; height: 30px; fill: none; stroke: var(--muted); stroke-width: 1.8; stroke-linecap: round; stroke-linejoin: round; }
  .label { display: block; font: 700 12px/1.2 var(--font-text); letter-spacing: 0.12em; text-transform: uppercase; color: var(--muted); }
  .value { display: block; font: 600 19px/1.25 var(--font-title); color: var(--text); }
</style>
