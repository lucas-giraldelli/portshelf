<script lang="ts">
  // A cartridge seen from the front: shell shape per console, cover art as the label.
  let { name, cover, console: consoleId, installed = true, size = 1 }: {
    name: string; cover?: string | null; console: string; installed?: boolean; size?: number;
  } = $props();
</script>

<div class="cart {consoleId}" class:missing={!installed} style="--s:{size}">
  <div class="ridges"></div>
  <div class="label">
    {#if cover}
      <img src={cover} alt={name} />
    {:else}
      <span>{name}</span>
    {/if}
  </div>
</div>

<style>
  .cart {
    --w: calc(150px * var(--s));
    width: var(--w);
    height: calc(var(--w) * 0.93);
    position: relative;
    background: linear-gradient(180deg, #4a4a4a, #2c2c2c);
    clip-path: polygon(6% 0, 94% 0, 100% 12%, 100% 100%, 0 100%, 0 12%);
    border-radius: 4px;
    box-shadow: inset 0 -6px 0 rgba(0, 0, 0, 0.35);
    transition: transform 0.15s ease, filter 0.15s ease;
  }
  .cart.snes { background: linear-gradient(180deg, #c9c9cf, #9d9da6); height: calc(var(--w) * 0.8); clip-path: none; border-radius: 10px 10px 4px 4px; }
  .cart.md { background: linear-gradient(180deg, #2a2a2a, #111); clip-path: none; border-radius: 6px; }
  .cart.gba { background: linear-gradient(180deg, #7d6fb8, #4b3c8f); height: calc(var(--w) * 0.62); clip-path: none; border-radius: 6px 6px 3px 3px; }
  .ridges {
    position: absolute; left: 10%; right: 10%; top: 4%; height: 10%;
    background: repeating-linear-gradient(90deg, rgba(255,255,255,0.08) 0 3px, transparent 3px 7px);
  }
  .label {
    position: absolute; left: 12%; right: 12%; top: 18%; bottom: 14%;
    background: #ddd; border-radius: 3px; overflow: hidden;
    display: grid; place-items: center; text-align: center;
    box-shadow: inset 0 0 0 2px rgba(0,0,0,0.25);
  }
  .label img { width: 100%; height: 100%; object-fit: cover; object-position: top; }
  .label span { font: 700 calc(12px * var(--s)) / 1.1 system-ui, sans-serif; color: #222; padding: 6px; }
  .missing { filter: grayscale(1) brightness(0.55); }
</style>
