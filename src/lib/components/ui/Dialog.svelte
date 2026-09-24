<script lang="ts">
  // Modal dialog: a scrim that closes it on click, and a panel with the content.
  import type { Snippet } from "svelte";
  let {
    onclose,
    labelledby,
    label,
    width = 380,
    align = "center",
    role = "dialog",
    children,
  }: {
    onclose: () => void;
    labelledby?: string;
    label?: string;
    width?: number;
    /** "center" for choices and forms, "top" for search. */
    align?: "center" | "top";
    role?: "dialog" | "alertdialog";
    children: Snippet;
  } = $props();
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="scrim" class:top={align === "top"} onclick={onclose}>
  <div class="box" {role} aria-modal="true" aria-labelledby={labelledby} aria-label={label} tabindex="-1" style="width: min({width}px, 94vw)" onclick={(e) => e.stopPropagation()}>
    {@render children()}
  </div>
</div>

<style>
  .scrim { position: fixed; inset: 0; z-index: 200; background: var(--scrim); display: grid; place-items: center; }
  .scrim.top { place-items: start center; padding-top: 12vh; }
  .box {
    position: relative; max-height: 88vh; overflow-y: auto; box-sizing: border-box;
    background: var(--panel); border: 1px solid var(--border); border-radius: 12px;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.6);
  }
</style>
