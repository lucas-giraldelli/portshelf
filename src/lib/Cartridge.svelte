<script lang="ts">
  // A cartridge seen from the front: a photographed shell whose label window is
  // transparent, with the cover art behind it. The art is shown whole (contain)
  // over a blurred copy of itself, so nothing of the original is cropped.
  let { name, cover, console: consoleId, shell, installed = true, size = 1 }: {
    name: string; cover?: string | null; console: string; shell?: string; installed?: boolean; size?: number;
  } = $props();

  // Shell image, its aspect ratio, and the label window as insets (%: left, top, right, bottom),
  // measured on the transparent area and grown slightly so the art tucks under the frame.
  const shells: Record<string, { src: string; aspect: number; width: number; window: [number, number, number, number] }> = {
    n64: { src: "/media/n64-cartridge.png", aspect: 916 / 593, width: 170, window: [26.6, 8.4, 25.4, 7.4] },
    snes: { src: "/media/snes-cartridge.png", aspect: 786 / 507, width: 180, window: [28.0, 0.8, 37.0, 63.2] },
    gba: { src: "/media/gba-cartridge.png", aspect: 528 / 300, width: 150, window: [10.8, 22.3, 10.2, 13.3] },
    md: { src: "/media/md-cartridge.png", aspect: 840 / 547, width: 170, window: [14.7, 6.9, 14.0, 15.3] },
  };
  let info = $derived(shells[consoleId] ?? shells.n64);
</script>

<div
  class="cart"
  class:missing={!installed}
  style="--s:{size}; --w:{info.width}px; --aspect:{info.aspect}; --shell:{shell ?? 'transparent'}; --src:url({info.src})"
>
  <div class="art" style="left:{info.window[0]}%; top:{info.window[1]}%; right:{info.window[2]}%; bottom:{info.window[3]}%">
    {#if cover}
      <img class="backdrop" src={cover} alt="" />
      <img class="front" src={cover} alt={name} />
    {:else}
      <span>{name}</span>
    {/if}
  </div>
  <img class="shell" src={info.src} alt="" draggable="false" />
  {#if shell}<div class="tint"></div>{/if}
</div>

<style>
  .cart {
    width: calc(var(--w) * var(--s));
    aspect-ratio: var(--aspect);
    position: relative;
    filter: drop-shadow(0 14px 14px rgba(0, 0, 0, 0.55));
  }
  .shell, .tint { position: absolute; inset: 0; width: 100%; height: 100%; }
  /* Recolours the shell (DK64 yellow, Zelda gold) while keeping its texture. */
  .tint {
    background: var(--shell);
    mix-blend-mode: overlay;
    -webkit-mask: var(--src) center / 100% 100% no-repeat;
    mask: var(--src) center / 100% 100% no-repeat;
  }
  .art { position: absolute; overflow: hidden; background: #d8d4cc; display: grid; place-items: center; }
  .art .backdrop { position: absolute; inset: -10%; width: 120%; height: 120%; object-fit: cover; filter: blur(8px) brightness(0.75); }
  .art .front { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: contain; }
  .art span { font: 700 calc(12px * var(--s)) / 1.1 system-ui, sans-serif; color: #222; padding: 6px; text-align: center; }
  .missing { filter: saturate(0.4) brightness(0.8) drop-shadow(0 14px 14px rgba(0, 0, 0, 0.55)); }
</style>
