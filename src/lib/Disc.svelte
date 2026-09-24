<script lang="ts">
  // An optical disc: cover art printed on it, a hub in the middle and a rainbow sheen.
  let { name, cover, console: consoleId, installed = true, size = 1 }: {
    name: string; cover?: string | null; console: string; installed?: boolean; size?: number;
  } = $props();
</script>

<div class="disc {consoleId}" class:missing={!installed} style="--s:{size}">
  <div class="print">
    {#if cover}
      <img src={cover} alt={name} />
    {:else}
      <span>{name}</span>
    {/if}
  </div>
  <div class="sheen"></div>
  <div class="hub"></div>
</div>

<style>
  .disc {
    --d: calc((var(--consoleSize, 150px)) * var(--s));
    width: var(--d); height: var(--d);
    border-radius: 50%; position: relative; overflow: hidden;
    background: #cfd3da;
    box-shadow: 0 2px 0 rgba(0,0,0,0.4), inset 0 0 0 2px rgba(255,255,255,0.4);
    transition: transform 0.15s ease, filter 0.15s ease;
  }
  .disc.gc { --consoleSize: 118px; }  /* mini disc */
  .print { position: absolute; inset: 0; display: grid; place-items: center; }
  .print img { width: 100%; height: 100%; object-fit: cover; }
  .print span { font: 700 calc(12px * var(--s)) / 1.1 system-ui, sans-serif; color: #222; max-width: 60%; text-align: center; transform: translateY(-28%); }
  .sheen {
    position: absolute; inset: 0; mix-blend-mode: overlay; opacity: 0.45;
    background: conic-gradient(from 30deg, #f0f 0deg, #0ff 60deg, #ff0 120deg, #f0f 180deg, #0ff 240deg, #ff0 300deg, #f0f 360deg);
  }
  .hub {
    position: absolute; left: 50%; top: 50%; width: 30%; height: 30%;
    transform: translate(-50%, -50%); border-radius: 50%;
    background: radial-gradient(circle, #1a1a1a 0 22%, #e6e8ec 23% 60%, rgba(200,205,215,0.9) 61%);
    box-shadow: 0 0 0 1px rgba(0,0,0,0.25);
  }
  .missing { filter: saturate(0.4) brightness(0.8); }
</style>
