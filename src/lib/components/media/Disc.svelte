<script lang="ts">
  // An optical disc: the cover art printed on the disc, under a template of that
  // system's disc (logos, bands and the hub drawn, label area and hole transparent).
  // The art is cut out of the centre hole, so the background shows through it.
  let { name, cover, console: consoleId, installed = true, size = 1 }: {
    name: string; cover?: string | null; console: string; installed?: boolean; size?: number;
  } = $props();

  // Template, diameter on the shelf, and the hole's radius as a fraction of the diameter
  // (15 mm hole: 120 mm CD/DVD, 80 mm GameCube mini-disc).
  const discs: Record<string, { src?: string; diameter: number; hole: number }> = {
    gc: { src: "/media/gc-disc.png", diameter: 125, hole: 15 / 80 / 2 },
    ps1: { src: "/media/ps1-disc.png", diameter: 150, hole: 15 / 120 / 2 },
    ps2: { src: "/media/ps2-disc.png", diameter: 150, hole: 15 / 120 / 2 },
    x360: { src: "/media/x360-disc.png", diameter: 150, hole: 15 / 120 / 2 },
    wii: { src: "/media/wii-disc.png", diameter: 150, hole: 15 / 120 / 2 },
  };
  let info = $derived(discs[consoleId] ?? { diameter: 150, hole: 15 / 120 / 2 });
</script>

<div class="disc" class:missing={!installed} style="--s:{size}; --d:{info.diameter}px; --hole:{info.hole * 100}%">
  <div class="print">
    {#if cover}
      <img src={cover} alt={name} />
    {:else}
      <span>{name}</span>
    {/if}
  </div>
  {#if info.src}
    <img class="template" src={info.src} alt="" draggable="false" />
  {:else}
    <div class="sheen"></div>
  {/if}
</div>

<style>
  .disc {
    width: calc(var(--d) * var(--s));
    aspect-ratio: 1;
    position: relative;
    filter: drop-shadow(0 12px 14px rgba(0, 0, 0, 0.55));
  }
  .print {
    position: absolute; inset: 1%; border-radius: 50%; overflow: hidden;
    background: #cfd3da; display: grid; place-items: center;
    -webkit-mask: radial-gradient(circle, transparent var(--hole), #000 calc(var(--hole) + 0.5%));
    mask: radial-gradient(circle, transparent var(--hole), #000 calc(var(--hole) + 0.5%));
  }
  .print img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; object-position: center 30%; }
  .print span { font: 700 calc(12px * var(--s)) / 1.1 "Outfit", system-ui, sans-serif; color: #222; max-width: 60%; text-align: center; transform: translateY(-28%); }
  .template { position: absolute; inset: 0; width: 100%; height: 100%; }
  .sheen {
    position: absolute; inset: 1%; border-radius: 50%; mix-blend-mode: overlay; opacity: 0.45;
    background: conic-gradient(from 30deg, #f0f 0deg, #0ff 60deg, #ff0 120deg, #f0f 180deg, #0ff 240deg, #ff0 300deg, #f0f 360deg);
  }
  .missing { filter: saturate(0.4) brightness(0.8) drop-shadow(0 12px 14px rgba(0, 0, 0, 0.55)); }
</style>
