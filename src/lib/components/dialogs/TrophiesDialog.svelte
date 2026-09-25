<script lang="ts">
  // Every game with achievements, grouped by system: total points on top, then each system's
  // points and its games' progress. Up and down (D-pad or arrows) choose a game, A or Enter
  // opens its achievements.
  import Modal from "$lib/components/ui/Modal.svelte";
  import TrophyIcon from "$lib/components/ui/TrophyIcon.svelte";
  import SystemLogo from "$lib/components/media/SystemLogo.svelte";
  import { tr } from "$lib/prefs.svelte";
  import { shelf } from "$lib/shelf.svelte";
  import { formatLastPlayed } from "$lib/playtime";
  import type { Summary } from "$lib/achievements";
  import type { Port } from "$lib/types";

  let { onclose, onopen, focusConsole }: { onclose: () => void; onopen: (port: Port) => void; focusConsole?: string } = $props();

  type Game = { summary: Summary; port: Port };
  let games = $derived(
    Object.values(shelf.achievements)
      .map((s) => ({ summary: s, port: shelf.portById(s.port) }))
      .filter((g): g is Game => !!g.port),
  );
  /** Systems in release order, each with its games (most recently unlocked first). */
  let groups = $derived(
    shelf.consoles
      .map(([id, info]) => {
        const list = games
          .filter((g) => g.port.console === id)
          .sort((a, b) => (b.summary.last ?? 0) - (a.summary.last ?? 0) || shelf.displayName(a.port).localeCompare(shelf.displayName(b.port)));
        return { id, info, games: list, points: list.reduce((n, g) => n + g.summary.points, 0), total: list.reduce((n, g) => n + g.summary.total_points, 0) };
      })
      .filter((g) => g.games.length > 0),
  );
  /** The games in the order they are shown, for moving the selection. */
  let flat = $derived(groups.flatMap((g) => g.games));
  let points = $derived(games.reduce((n, g) => n + g.summary.points, 0));
  let unlocked = $derived(games.reduce((n, g) => n + g.summary.unlocked, 0));
  let total = $derived(games.reduce((n, g) => n + g.summary.total, 0));

  let index = $state(0);
  let rows: Record<string, HTMLButtonElement> = $state({});
  // Opened from a system on the shelf: start on that system's first game.
  $effect(() => {
    const first = flat.findIndex((g) => g.port.console === focusConsole);
    if (first > 0) select(first);
  });
  function select(i: number) {
    index = Math.max(0, Math.min(flat.length - 1, i));
    rows[flat[index]?.port.id]?.scrollIntoView({ block: "nearest" });
  }

  export function handlePad(action: string) {
    if (action === "down") select(index + 1);
    else if (action === "up") select(index - 1);
    else if (action === "a" && flat[index]) onopen(flat[index].port);
    else return false;
    return true;
  }
  export function handleKey(e: KeyboardEvent) {
    const action = ({ ArrowDown: "down", ArrowUp: "up", Enter: "a" } as Record<string, string>)[e.key];
    return action ? handlePad(action) : false;
  }
</script>

<Modal title={tr("ach.title")} label={tr("ach.title")} {onclose}>
  <div class="totals">
    <span class="tile"><TrophyIcon size={34} /></span>
    <div>
      <span class="big">{points}</span> <span class="unit">{tr("ach.points")}</span>
      <p>{tr("ach.count", { n: unlocked, total })}</p>
    </div>
  </div>
  {#each groups as group (group.id)}
    <section class="sys">
      <h3 class="sys-head">
        <SystemLogo id={group.id} name={group.info.name} height={20} />
        <span class="sys-pts">{group.points}<small>/{group.total} {tr("ach.points")}</small></span>
      </h3>
      <ul class="games">
        {#each group.games as g (g.port.id)}
          <li class="game">
            <button class="game-row" bind:this={rows[g.port.id]} class:on={flat[index]?.port.id === g.port.id} onclick={() => onopen(g.port)} onmouseenter={() => (index = flat.indexOf(g))}>
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
        {/each}
      </ul>
    </section>
  {:else}
    <p class="none">{tr("ach.none")}</p>
  {/each}
</Modal>

<style>
  .totals { display: flex; align-items: center; gap: 16px; margin-bottom: 8px; }
  .tile { width: 64px; height: 64px; border-radius: 14px; display: grid; place-items: center; background: var(--accent); color: var(--on-accent); }
  .big { font: 800 36px/1 var(--font-title); color: var(--accent); }
  .unit { color: var(--muted); font-size: 16px; }
  .totals p { margin: 4px 0 0; color: var(--muted); }
  .sys { margin-top: 16px; }
  .sys-head { display: flex; align-items: center; justify-content: space-between; margin: 0 0 8px; color: var(--text); }
  .sys-pts { font: 700 16px var(--font-title); color: var(--accent); }
  .sys-pts small, .pts small { color: var(--muted); font-size: 13px; font-weight: 600; }
  .games { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; }
  .game-row { width: 100%; display: flex; align-items: center; gap: 14px; padding: 10px 12px; border: 2px solid transparent; border-radius: var(--radius); background: var(--surface); text-align: left; }
  .game-row.on { background: var(--surface-2); border-color: var(--accent); }
  .thumb { width: 64px; height: 46px; flex: none; border-radius: 6px; overflow: hidden; background: var(--surface-2); }
  .thumb img { width: 100%; height: 100%; object-fit: cover; }
  .info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 5px; }
  .name { font: 700 17px/1.2 var(--font-title); }
  .bar { height: 6px; border-radius: 3px; background: var(--surface-2); overflow: hidden; }
  .bar span { display: block; height: 100%; background: var(--accent); }
  .meta { color: var(--muted); font-size: 14px; }
  .pts { font: 700 20px var(--font-title); color: var(--accent); }
  .none { color: var(--muted); }
</style>
