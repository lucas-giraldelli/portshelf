<script lang="ts">
  // "Quit PortShelf?" with No focused, answered with the mouse, the keyboard or the controller.
  import Dialog from "$lib/components/ui/Dialog.svelte";
  import ChoiceHints from "$lib/components/ui/ChoiceHints.svelte";
  import { tr } from "$lib/prefs.svelte";

  let { pad, onanswer }: { pad: boolean; onanswer: (yes: boolean) => void } = $props();
  let yes = $state(false);

  export function handleKey(e: KeyboardEvent) {
    if (["ArrowLeft", "ArrowRight", "a", "d", "Tab"].includes(e.key)) yes = !yes;
    else if (e.key === "Enter" || e.key === " ") onanswer(yes);
    else if (e.key === "Escape" || e.key === "Backspace" || e.key.toLowerCase() === "n") onanswer(false);
    else if (e.key.toLowerCase() === "y") onanswer(true);
    return true;
  }
  export function handlePad(action: string) {
    if (action === "left" || action === "right") yes = !yes;
    else if (action === "a") onanswer(yes);
    else if (action === "b") onanswer(false);
    return true;
  }
</script>

<Dialog onclose={() => onanswer(false)} labelledby="quit-title" role="alertdialog" width={440}>
  <div class="body">
    <h2 id="quit-title">{tr("quit.title")}</h2>
    <div class="choices">
      <button class="tool" class:on={!yes} onclick={() => onanswer(false)} onmouseenter={() => (yes = false)}>{tr("common.no")}</button>
      <button class="tool" class:on={yes} onclick={() => onanswer(true)} onmouseenter={() => (yes = true)}>{tr("common.yes")}</button>
    </div>
    <ChoiceHints {pad} />
  </div>
</Dialog>

<style>
  .body { padding: 22px 22px 14px; text-align: center; }
  h2 { margin: 0 0 18px; font-size: 20px; }
  .choices { display: flex; gap: 12px; justify-content: center; }
  .choices .tool { min-width: 110px; }
</style>
