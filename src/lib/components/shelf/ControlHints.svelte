<script lang="ts">
  // Footer with the controls of whatever was used last: controller, or keyboard and mouse.
  import PadGlyph from "$lib/components/ui/PadGlyph.svelte";
  import { tr } from "$lib/prefs.svelte";

  let { view, pad, primary, showAll }: { view: "systems" | "games"; pad: boolean; primary: string; showAll: boolean } = $props();
  let games = $derived(view === "games");
  let toggle = $derived(showAll ? tr("footer.installedOnly") : tr("footer.everyPort"));
</script>

<footer class="hints">
  {#if pad}
    <span><PadGlyph button="east" /> {games ? tr("footer.systems") : tr("footer.quit")}</span>
    <span><PadGlyph button="dpad" /> {tr("footer.browse")}</span>
    <span><PadGlyph button="south" /> {primary}</span>
    <span><PadGlyph button="north" /> {tr("footer.achievements")}</span>
    <span><PadGlyph button="west" /> {tr("footer.known")}</span>
    <span><kbd class="pad">L2</kbd> {games ? tr("footer.settings") : tr("footer.options")}</span>
    <span><kbd class="pad">R2</kbd> {tr("footer.fullscreen")}</span>
    {#if games}<span><kbd class="pad">LB</kbd><kbd class="pad">RB</kbd> {tr("footer.system")}</span>{/if}
    <span><kbd class="pad">Start</kbd> {tr("footer.search")}</span>
    <span><kbd class="pad">Select</kbd> {toggle}</span>
  {:else}
    <span><kbd>Esc</kbd> {games ? tr("footer.systems") : tr("footer.quit")}</span>
    {#if !games}<span><kbd>O</kbd> {tr("footer.options")}</span>{/if}
    <span><kbd>←</kbd><kbd>→</kbd> {tr("footer.browse")}</span>
    <span><kbd>Enter</kbd> {primary}</span>
    {#if games}
      <span><kbd>S</kbd> {tr("footer.settings")}</span>
      <span><kbd>R</kbd> {tr("footer.rename")}</span>
    {/if}
    <span><kbd>C</kbd> {tr("footer.achievements")}</span>
    <span><kbd>K</kbd> {tr("footer.known")}</span>
    <span><kbd>Ctrl</kbd><kbd>F</kbd> {tr("footer.search")}</span>
    <span><kbd>Tab</kbd> {toggle}</span>
  {/if}
</footer>

<style>
  footer { font-size: 15px; padding: 10px 0 6px; }
</style>
