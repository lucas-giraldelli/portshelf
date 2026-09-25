<script lang="ts">
  // A game's achievements: unlocked ones first with the day they were earned, locked ones
  // dimmed, and the points of each. Up and down (D-pad or arrows) move through the list.
  import { invoke } from "@tauri-apps/api/core";
  import Modal from "$lib/components/ui/Modal.svelte";
  import TrophyIcon from "$lib/components/ui/TrophyIcon.svelte";
  import { tr } from "$lib/prefs.svelte";
  import { shelf } from "$lib/shelf.svelte";
  import { localized, type AchievementStatus } from "$lib/achievements";
  import { formatLastPlayed } from "$lib/playtime";
  import type { Port } from "$lib/types";

  let { port, onclose }: { port: Port; onclose: () => void } = $props();

  let list = $state<AchievementStatus[]>([]);
  $effect(() => {
    invoke<AchievementStatus[]>("get_achievements", { port: port.id })
      .then((l) => (list = l))
      .catch((e) => shelf.fail(e));
  });
  let sorted = $derived([...list].sort((a, b) => Number(!!b.unlocked) - Number(!!a.unlocked) || (b.unlocked ?? 0) - (a.unlocked ?? 0)));
  let summary = $derived(shelf.achievements[port.id]);

  let index = $state(0);
  let rows: HTMLLIElement[] = $state([]);
  function select(i: number) {
    index = Math.max(0, Math.min(sorted.length - 1, i));
    rows[index]?.scrollIntoView({ block: "nearest" });
  }
  export function handlePad(action: string) {
    if (action === "down") select(index + 1);
    else if (action === "up") select(index - 1);
    else return false;
    return true;
  }
  export function handleKey(e: KeyboardEvent) {
    const action = ({ ArrowDown: "down", ArrowUp: "up" } as Record<string, string>)[e.key];
    return action ? handlePad(action) : false;
  }
</script>

<Modal title={shelf.displayName(port)} label={tr("ach.aria", { name: shelf.displayName(port) })} {onclose}>
  {#snippet header()}
    {#if summary}
      <span class="score">
        <strong>{summary.points}</strong>/{summary.total_points} {tr("ach.points")} · {tr("ach.count", { n: summary.unlocked, total: summary.total })}
      </span>
    {/if}
  {/snippet}
  <ul>
    {#each sorted as a, i (a.id)}
      <li bind:this={rows[i]} class:locked={!a.unlocked} class:on={i === index} onmouseenter={() => (index = i)}>
        <span class="tile"><TrophyIcon locked={!a.unlocked} /></span>
        <span class="text">
          <span class="title">{localized(a.title)}</span>
          <span class="desc">{localized(a.description)}</span>
          {#if a.unlocked}<span class="when">{tr("ach.unlockedOn", { when: formatLastPlayed(a.unlocked) })}</span>{/if}
        </span>
        <span class="pts">+{a.points}</span>
      </li>
    {:else}
      <li class="none">{tr("ach.none")}</li>
    {/each}
  </ul>
</Modal>

<style>
  .score { color: var(--muted); font-size: 15px; margin-right: 6px; }
  .score strong { color: var(--accent); font-size: 18px; }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; }
  li { display: flex; align-items: center; gap: 14px; padding: 10px 12px; border: 2px solid transparent; border-radius: var(--radius); background: var(--surface); }
  li.on { border-color: var(--accent); }
  .tile { width: 48px; height: 48px; flex: none; border-radius: 10px; display: grid; place-items: center; background: var(--accent); color: var(--on-accent); }
  .locked .tile { background: var(--surface-2); color: var(--muted); }
  .text { display: flex; flex-direction: column; gap: 2px; flex: 1; min-width: 0; }
  .title { font: 700 17px/1.2 var(--font-title); }
  .desc { color: var(--muted); font-size: 15px; }
  .when { color: var(--ok); font-size: 13px; }
  .pts { font: 700 17px var(--font-title); color: var(--accent); }
  .locked .title, .locked .pts { color: var(--muted); }
  .none { color: var(--muted); background: none; }
</style>
