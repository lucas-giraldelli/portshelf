<script lang="ts">
  // Brand on the left; settings and fullscreen on the right.
  import { prefs, tr } from "$lib/prefs.svelte";
  import { toggleFullscreen } from "$lib/fullscreen";
  import TrophyIcon from "$lib/components/ui/TrophyIcon.svelte";
  let { onsettings, ontrophies }: { onsettings: () => void; ontrophies: () => void } = $props();
</script>

<header>
  <h1><img src="/icon.svg" alt="" width="34" height="34" draggable="false" />PortShelf</h1>
  <button class="tool square" onclick={ontrophies} title={tr("ach.title")} aria-label={tr("ach.title")}><TrophyIcon size={20} /></button>
  <button class="tool square" onclick={onsettings} title={tr("settings.open")} aria-label={tr("settings.open")}>⚙</button>
  <button class="tool square" onclick={toggleFullscreen} title={tr(prefs.fullscreen ? "header.leaveFullscreen" : "header.fullscreen")} aria-label={tr(prefs.fullscreen ? "header.leaveFullscreen" : "header.fullscreen")}>
    <svg viewBox="0 0 24 24" aria-hidden="true">
      {#if prefs.fullscreen}
        <path d="M9 4v5H4M15 4v5h5M9 20v-5H4M15 20v-5h5" />
      {:else}
        <path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5" />
      {/if}
    </svg>
  </button>
</header>

<style>
  header { display: flex; align-items: center; gap: 10px; }
  h1 {
    flex: 1; margin: 4px 0; display: flex; align-items: center; gap: 10px;
    font-size: 22px; font-weight: 800; letter-spacing: 0.08em; text-transform: uppercase;
  }
  h1 img { display: block; }
  .tool { font-size: 19px; }
  svg { width: 20px; height: 20px; fill: none; stroke: currentColor; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }
</style>
