<script lang="ts">
  // Brand on the left, the settings gear on the right.
  import { tr } from "$lib/prefs.svelte";
  let { onsettings }: { onsettings: () => void } = $props();
</script>

<header>
  <h1><img src="/icon.svg" alt="" width="34" height="34" draggable="false" />PortShelf</h1>
  <button class="tool square" onclick={onsettings} title={tr("settings.open")} aria-label={tr("settings.open")}>⚙</button>
</header>

<style>
  header { display: flex; align-items: center; gap: 20px; }
  h1 {
    flex: 1; margin: 4px 0; display: flex; align-items: center; gap: 10px;
    font-size: 22px; font-weight: 800; letter-spacing: 0.08em; text-transform: uppercase;
  }
  h1 img { display: block; }
  .tool { font-size: 19px; }
</style>
