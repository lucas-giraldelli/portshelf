<script lang="ts">
  // Key hints under a yes / no choice, for the controller or the keyboard.
  import PadGlyph from "./PadGlyph.svelte";
  import { tr } from "$lib/prefs.svelte";
  let { pad }: { pad: boolean } = $props();
</script>

<p class="hints">
  {#if pad}
    <span><PadGlyph button="dpad" /> {tr("footer.choose")}</span>
    <span><PadGlyph button="south" /> {tr("footer.confirm")}</span>
    <span><PadGlyph button="east" /> {tr("footer.cancel")}</span>
  {:else}
    <span><kbd>←</kbd><kbd>→</kbd> {tr("footer.choose")}</span>
    <span><kbd>Enter</kbd> {tr("footer.confirm")}</span>
    <span><kbd>Esc</kbd> {tr("footer.cancel")}</span>
  {/if}
</p>

<style>
  .hints { margin: 16px 0 0; font-size: 14px; }
</style>
