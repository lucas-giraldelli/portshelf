<script lang="ts">
  // Console hardware: a photo when one is bundled (static/consoles, see CREDITS.md),
  // otherwise a stylised SVG drawn at a 200x160 viewBox.
  let { id, size = 200 }: { id: string; size?: number } = $props();
  const photos = new Set(["n64", "gc", "snes", "gba", "ps1", "ps2", "md", "x360", "nes", "gb", "wii", "arcade"]);
</script>

{#if photos.has(id)}
  <img class="photo" src="/consoles/{id}.png" alt="" style="max-width: {size}px; max-height: {size * 0.8}px" draggable="false" />
{:else}
<svg viewBox="0 0 200 160" width={size} height={size * 0.8} aria-hidden="true">
  {#if id === "gc"}
    <!-- GameCube: indigo cube with the handle at the back and the disc lid on top -->
    <rect x="44" y="20" width="112" height="12" rx="6" fill="#3d3488" />
    <rect x="40" y="30" width="120" height="112" rx="10" fill="#5a4fcf" />
    <rect x="40" y="30" width="120" height="112" rx="10" fill="url(#gcShade)" />
    <circle cx="100" cy="74" r="30" fill="#4a40b5" stroke="#3d3488" stroke-width="3" />
    <circle cx="100" cy="74" r="6" fill="#2d2670" />
    <rect x="56" y="120" width="22" height="8" rx="2" fill="#2d2670" />
    <rect x="122" y="120" width="22" height="8" rx="2" fill="#2d2670" />
    <defs>
      <linearGradient id="gcShade" x1="0" x2="1">
        <stop offset="0" stop-color="#fff" stop-opacity="0.12" />
        <stop offset="1" stop-color="#000" stop-opacity="0.18" />
      </linearGradient>
    </defs>
  {:else if id === "n64"}
    <!-- Nintendo 64: dark body, raised cartridge slot, four controller ports -->
    <path d="M18 70 Q20 52 40 50 L160 50 Q180 52 182 70 L186 122 Q186 134 172 134 L28 134 Q14 134 14 122 Z" fill="#2f2f33" />
    <rect x="62" y="34" width="76" height="26" rx="4" fill="#232326" />
    <rect x="70" y="40" width="60" height="8" rx="2" fill="#111" />
    <rect x="30" y="72" width="22" height="12" rx="3" fill="#3f3f45" />
    <rect x="148" y="72" width="22" height="12" rx="3" fill="#3f3f45" />
    {#each [52, 80, 108, 136] as x}
      <rect x={x} y="112" width="14" height="12" rx="2" fill="#18181a" />
    {/each}
    <circle cx="100" cy="90" r="7" fill="#c33" />
  {:else if id === "snes"}
    <!-- Super Nintendo: grey body, purple switches, cartridge slot -->
    <rect x="20" y="52" width="160" height="80" rx="16" fill="#c9c9cf" />
    <rect x="58" y="40" width="84" height="24" rx="6" fill="#aeaeb6" />
    <rect x="66" y="46" width="68" height="6" rx="2" fill="#6e6e78" />
    <rect x="40" y="86" width="26" height="14" rx="3" fill="#7b5fb3" />
    <rect x="134" y="86" width="26" height="14" rx="3" fill="#a797cf" />
    <rect x="30" y="118" width="140" height="6" rx="3" fill="#9d9da6" />
  {:else if id === "gba"}
    <!-- Game Boy Advance -->
    <rect x="14" y="40" width="172" height="90" rx="36" fill="#4b3c8f" />
    <rect x="58" y="52" width="84" height="62" rx="6" fill="#1d1d24" />
    <rect x="66" y="58" width="68" height="50" rx="2" fill="#9fb58a" />
    <circle cx="36" cy="84" r="12" fill="#2d2466" />
    <circle cx="158" cy="78" r="7" fill="#2d2466" />
    <circle cx="170" cy="92" r="7" fill="#2d2466" />
  {:else if id === "ps1"}
    <!-- PlayStation: light grey body, round lid -->
    <rect x="22" y="44" width="156" height="92" rx="8" fill="#c7c7c9" />
    <circle cx="112" cy="84" r="36" fill="#b8b8bb" stroke="#a3a3a7" stroke-width="3" />
    <rect x="34" y="112" width="30" height="10" rx="2" fill="#8e8e93" />
    <rect x="34" y="96" width="14" height="10" rx="2" fill="#6b6b70" />
    <rect x="52" y="96" width="14" height="10" rx="2" fill="#6b6b70" />
  {:else if id === "ps2"}
    <!-- PlayStation 2: black tower, horizontal grooves, blue logo -->
    <rect x="30" y="34" width="140" height="102" rx="6" fill="#16171d" />
    {#each [52, 70, 88, 106] as y}
      <rect x="44" y={y} width="112" height="3" fill="#2a2c36" />
    {/each}
    <rect x="44" y="42" width="112" height="6" rx="2" fill="#0b0c10" />
    <rect x="140" y="118" width="16" height="6" rx="2" fill="#2f5bd8" />
  {:else if id === "md"}
    <!-- Mega Drive: black body, circular top, red slider -->
    <rect x="16" y="60" width="168" height="72" rx="10" fill="#1a1a1a" />
    <circle cx="100" cy="72" r="36" fill="#232323" stroke="#0e0e0e" stroke-width="3" />
    <rect x="70" y="46" width="60" height="10" rx="2" fill="#0e0e0e" />
    <rect x="30" y="112" width="20" height="8" rx="2" fill="#c22" />
  {:else if id === "x360"}
    <!-- Xbox 360: white body, green ring -->
    <rect x="30" y="30" width="140" height="104" rx="16" fill="#e8e8ea" />
    <rect x="44" y="44" width="112" height="76" rx="10" fill="#d6d6d9" />
    <circle cx="100" cy="82" r="14" fill="none" stroke="#52b043" stroke-width="5" />
  {:else}
    <rect x="30" y="40" width="140" height="90" rx="12" fill="#555" />
  {/if}
</svg>
{/if}

<style>
  .photo { display: block; object-fit: contain; filter: drop-shadow(0 18px 18px rgba(0, 0, 0, 0.55)); }
</style>
