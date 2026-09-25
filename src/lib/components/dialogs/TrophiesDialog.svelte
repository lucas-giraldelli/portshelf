<script lang="ts">
  // Every game with achievements: total points, how many are unlocked, and each game's progress.
  // Choosing a game opens its achievements.
  import Modal from "$lib/components/ui/Modal.svelte";
  import TrophyIcon from "$lib/components/ui/TrophyIcon.svelte";
  import { tr } from "$lib/prefs.svelte";
  import { shelf } from "$lib/shelf.svelte";
  import { formatLastPlayed } from "$lib/playtime";
  import type { Port } from "$lib/types";

  let { onclose, onopen }: { onclose: () => void; onopen: (port: Port) => void } = $props();

  let games = $derived(
    Object.values(shelf.achievements)
      .map((s) => ({ summary: s, port: shelf.portById(s.port) }))
      .filter((g): g is { summary: typeof g.summary; port: Port } => !!g.port)
      .sort((a, b) => (b.summary.last ?? 0) - (a.summary.last ?? 0) || shelf.displayName(a.port).localeCompare(shelf.displayName(b.port))),
  );
  let points = $derived(games.reduce((n, g) => n + g.summary.points, 0));
  let unlocked = $derived(games.reduce((n, g) => n + g.summary.unlocked, 0));
  let total = $derived(games.reduce((n, g) => n + g.summary.total, 0));
</script>

<Modal title={tr("ach.title")} label={tr("ach.title")} {onclose}>
  <div class="totals">
    <span class="tile"><TrophyIcon size={34} /></span>
    <div>
      <span class="big">{points}</span> <span class="unit">{tr("ach.points")}</span>
      <p>{tr("ach.count", { n: unlocked, total })}</p>
    </div>
  </div>
  <ul>
    {#each games as g (g.port.id)}
      <li>
        <button onclick={() => onopen(g.port)}>
          <span class="thumb">{#if shelf.covers[g.port.id]}<img src={shelf.covers[g.port.id]} alt="" />{/if}</span>
          <span class="info">
            <span class="name">{shelf.displayName(g.port)}</span>
            <span class="bar"><span style="width:{(g.summary.unlocked / Math.max(1, g.summary.total)) * 100}%"></span></span>
            <span class="meta">
              {tr("ach.count", { n: g.summary.unlocked, total: g.summary.total })}
              {#if g.summary.last} · {tr("ach.lastUnlock", { when: formatLastPlayed(g.summary.last) })}{/if}
            </span>
          </span>
          <span class="pts">{g.summary.points}<small>/{g.summary.total_points}</small></span>
        </button>
      </li>
    {:else}
      <li class="none">{tr("ach.none")}</li>
    {/each}
  </ul>
</Modal>

<style>
  .totals { display: flex; align-items: center; gap: 16px; margin-bottom: 18px; }
  .tile { width: 64px; height: 64px; border-radius: 14px; display: grid; place-items: center; background: var(--accent); color: var(--on-accent); }
  .big { font: 800 36px/1 var(--font-title); color: var(--accent); }
  .unit { color: var(--muted); font-size: 16px; }
  .totals p { margin: 4px 0 0; color: var(--muted); }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; }
  button { width: 100%; display: flex; align-items: center; gap: 14px; padding: 10px 12px; border: 0; border-radius: var(--radius); background: var(--surface); text-align: left; }
  button:hover, button:focus-visible { background: var(--surface-2); }
  .thumb { width: 64px; height: 46px; flex: none; border-radius: 6px; overflow: hidden; background: var(--surface-2); }
  .thumb img { width: 100%; height: 100%; object-fit: cover; }
  .info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 5px; }
  .name { font: 700 17px/1.2 var(--font-title); }
  .bar { height: 6px; border-radius: 3px; background: var(--surface-2); overflow: hidden; }
  .bar span { display: block; height: 100%; background: var(--accent); }
  .meta { color: var(--muted); font-size: 14px; }
  .pts { font: 700 20px var(--font-title); color: var(--accent); }
  .pts small { color: var(--muted); font-size: 14px; }
  .none { color: var(--muted); }
</style>
