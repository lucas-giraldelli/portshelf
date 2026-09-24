<script lang="ts" generics="T">
  // Horizontal carousel around a focused item. Keyboard and controller move it from the page;
  // here the mouse can drag it sideways, scroll it with the wheel, or click an item.
  import type { Snippet } from "svelte";

  let {
    items,
    key,
    focused,
    spacing,
    loop = false,
    label,
    onmove,
    onfocus,
    onactivate,
    item,
  }: {
    items: T[];
    key: (item: T) => string;
    focused: number;
    /** Distance between neighbouring items, in pixels. */
    spacing: number;
    /** Items wrap around, each one sitting on whichever side of the focus is closer. */
    loop?: boolean;
    label: string;
    onmove: (delta: number) => void;
    onfocus: (index: number) => void;
    /** Click on the focused item. */
    onactivate: () => void;
    item: Snippet<[T, number]>;
  } = $props();

  const wrap = (i: number, n: number) => (n ? ((i % n) + n) % n : 0);

  // Position of item i relative to the focused one.
  function slot(i: number) {
    let offset = i - focused;
    const n = items.length;
    if (loop && n > 2) offset = wrap(offset + Math.floor(n / 2), n) - Math.floor(n / 2);
    const distance = Math.abs(offset);
    return `transform: translate(calc(-50% + ${offset * spacing}px), -50%) scale(${Math.max(0.55, 1 - distance * 0.18)}); opacity: ${Math.max(0, 1 - distance * 0.28)}; z-index: ${100 - distance};`;
  }

  // A drag of about one item's width moves one step; a drag never counts as a click.
  let drag: { x: number; moved: boolean } | null = null;
  let suppressClick = false;
  function dragStart(e: PointerEvent) {
    if (e.button === 0) drag = { x: e.clientX, moved: false };
  }
  function dragMove(e: PointerEvent) {
    if (!drag) return;
    const dx = e.clientX - drag.x;
    if (Math.abs(dx) >= spacing * 0.5) {
      onmove(dx < 0 ? 1 : -1);
      drag.x = e.clientX;
      drag.moved = true;
    }
  }
  function dragEnd() {
    if (drag?.moved) {
      suppressClick = true;
      setTimeout(() => (suppressClick = false), 0);
    }
    drag = null;
  }

  let wheelAt = 0;
  function onWheel(e: WheelEvent) {
    const delta = Math.abs(e.deltaX) > Math.abs(e.deltaY) ? e.deltaX : e.deltaY;
    if (Math.abs(delta) < 4 || Date.now() - wheelAt < 120) return;
    wheelAt = Date.now();
    onmove(delta > 0 ? 1 : -1);
  }

  function click(i: number) {
    if (suppressClick) return;
    if (i === focused) onactivate();
    else onfocus(i);
  }
</script>

<section aria-label={label} onpointerdown={dragStart} onpointermove={dragMove} onpointerup={dragEnd} onpointerleave={dragEnd} onwheel={onWheel}>
  {#each items as it, i (key(it))}
    <button class="slot" class:focused={i === focused} style={slot(i)} onclick={() => click(i)}>
      {@render item(it, i)}
    </button>
  {/each}
</section>

<style>
  section { position: relative; flex: 1; min-height: 280px; touch-action: pan-y; cursor: grab; }
  section:active { cursor: grabbing; }
  .slot {
    position: absolute; left: 50%; top: 50%; transform-origin: 50% 50%;
    will-change: transform, opacity; /* own compositing layer: no repaint flicker while moving */
    background: none; border: 0; padding: 0;
    transition: transform 0.28s cubic-bezier(0.2, 0.8, 0.2, 1), opacity 0.28s ease;
  }
  .slot:focus-visible { outline: 2px solid var(--accent); outline-offset: 8px; border-radius: 12px; }

  /* Focused items lift smoothly (a transition, so leaving focus never snaps back), and discs
     spin only while focused: pausing keeps their angle instead of jumping back to 0. */
  .slot > :global(*) { transition: translate 0.35s cubic-bezier(0.2, 0.8, 0.2, 1); }
  .slot.focused > :global(*) { translate: 0 -12px; }
  .slot :global(.disc) { animation: spin 9s linear infinite paused; }
  .slot.focused :global(.disc) { animation-play-state: running; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) {
    .slot, .slot > :global(*), .slot :global(.disc) { animation: none; transition: none; }
  }
</style>
