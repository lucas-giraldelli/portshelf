<script lang="ts">
  // A centred dialog with a title, a close button and a scrolling body: known ports, a port's
  // settings, achievements.
  import type { Snippet } from "svelte";
  import Dialog from "./Dialog.svelte";
  import { tr } from "$lib/prefs.svelte";

  let {
    title,
    label,
    onclose,
    width = 640,
    header,
    children,
  }: { title: string; label: string; onclose: () => void; width?: number; header?: Snippet; children: Snippet } = $props();
</script>

<Dialog {onclose} {label} {width}>
  <div class="modal">
    <div class="head">
      <h2>{title}</h2>
      {@render header?.()}
      <button class="close" onclick={onclose} aria-label={tr("common.close")}>×</button>
    </div>
    <div class="body">
      {@render children()}
    </div>
  </div>
</Dialog>

<style>
  .modal { display: flex; flex-direction: column; max-height: 84vh; }
  .head { display: flex; align-items: center; gap: 14px; padding: 20px 24px 14px; border-bottom: 1px solid var(--border); }
  h2 { margin: 0; font-size: 22px; flex: 1; }
  .head .close { position: static; }
  .body { padding: 16px 24px 22px; overflow-y: auto; }
</style>
