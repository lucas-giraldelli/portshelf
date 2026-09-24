<script lang="ts">
  // A cartridge seen from the front. N64 uses a photographed shell with the label
  // window cut out; the other systems use a CSS shell. The cover art is shown whole
  // (contain) over a blurred copy of itself, so nothing of the original art is cropped.
  let { name, cover, console: consoleId, shell, installed = true, size = 1 }: {
    name: string; cover?: string | null; console: string; shell?: string; installed?: boolean; size?: number;
  } = $props();
</script>

{#if consoleId === "n64"}
  <div class="n64" class:missing={!installed} style="--s:{size}; --shell:{shell ?? 'transparent'}">
    <div class="art">
      {#if cover}
        <img class="backdrop" src={cover} alt="" />
        <img class="front" src={cover} alt={name} />
      {:else}
        <span>{name}</span>
      {/if}
    </div>
    <img class="shell" src="/media/n64-cartridge.png" alt="" draggable="false" />
    {#if shell}<div class="tint"></div>{/if}
  </div>
{:else}
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
{/if}

<style>
  /* Shell photo is 916x593; its label window spans 27.3%-73.9% x 9.1%-91.9%. */
  .n64 {
    --w: calc(170px * var(--s));
    width: var(--w);
    aspect-ratio: 916 / 593;
    position: relative;
    filter: drop-shadow(0 14px 14px rgba(0, 0, 0, 0.55));
  }
  .n64 .shell, .n64 .tint { position: absolute; inset: 0; width: 100%; height: 100%; }
  .n64 .tint {
    background: var(--shell);
    mix-blend-mode: overlay;
    -webkit-mask: url(/media/n64-cartridge.png) center / 100% 100% no-repeat;
    mask: url(/media/n64-cartridge.png) center / 100% 100% no-repeat;
  }
  .n64 .art {
    position: absolute; left: 26.6%; top: 8.4%; right: 25.4%; bottom: 7.4%;
    overflow: hidden; background: #d8d4cc; display: grid; place-items: center;
  }
  .n64 .art .backdrop { position: absolute; inset: -10%; width: 120%; height: 120%; object-fit: cover; filter: blur(8px) brightness(0.75); }
  .n64 .art .front { position: relative; width: 100%; height: 100%; object-fit: contain; }
  .n64 .art span { font: 700 calc(12px * var(--s)) / 1.1 system-ui, sans-serif; color: #222; padding: 6px; text-align: center; }

  .cart {
    --w: calc(150px * var(--s));
    width: var(--w);
    height: calc(var(--w) * 0.93);
    position: relative;
    background: linear-gradient(180deg, #4a4a4a, #2c2c2c);
    border-radius: 4px;
    box-shadow: inset 0 -6px 0 rgba(0, 0, 0, 0.35);
  }
  .cart.snes { background: linear-gradient(180deg, #c9c9cf, #9d9da6); height: calc(var(--w) * 0.8); border-radius: 10px 10px 4px 4px; }
  .cart.md { background: linear-gradient(180deg, #2a2a2a, #111); border-radius: 6px; }
  .cart.gba { background: linear-gradient(180deg, #7d6fb8, #4b3c8f); height: calc(var(--w) * 0.62); border-radius: 6px 6px 3px 3px; }
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
  .label img { width: 100%; height: 100%; object-fit: contain; }
  .label span { font: 700 calc(12px * var(--s)) / 1.1 system-ui, sans-serif; color: #222; padding: 6px; }
  .missing { filter: saturate(0.4) brightness(0.8); }
</style>
