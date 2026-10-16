<script lang="ts">
  // Frame-rate overlay for diagnosing a slow WebView, toggled with F3. Shows the frames drawn in
  // the last half second and the slowest frame in that window. F4 switches off the costly paint
  // effects (shadows, blurs, blend modes, the backdrop gradient) to measure what they cost.
  const KEY = "portshelf.fps";

  function stored(): boolean {
    try { return localStorage.getItem(KEY) === "1"; } catch { return false; }
  }

  let visible = $state(stored());
  let fps = $state(0);
  let worst = $state(0);
  let lite = $state(false);

  function onKey(e: KeyboardEvent) {
    if (e.key === "F3") {
      e.preventDefault();
      visible = !visible;
      try { localStorage.setItem(KEY, visible ? "1" : "0"); } catch {}
    } else if (e.key === "F4") {
      e.preventDefault();
      lite = !lite;
      document.documentElement.classList.toggle("lite", lite);
    }
  }

  $effect(() => {
    if (!visible) return;
    let frames = 0, slowest = 0, last = performance.now(), windowStart = last, raf = 0;
    const tick = (now: number) => {
      frames++;
      slowest = Math.max(slowest, now - last);
      last = now;
      if (now - windowStart >= 500) {
        fps = Math.round((frames * 1000) / (now - windowStart));
        worst = Math.round(slowest);
        frames = 0; slowest = 0; windowStart = now;
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  });
</script>

<svelte:window onkeydown={onKey} />

{#if visible}
  <div class="fps">
    <strong>{fps}</strong> fps · worst {worst} ms
    <span>effects {lite ? "off" : "on"} (F4)</span>
  </div>
{/if}

<style>
  .fps {
    position: fixed; left: 12px; bottom: 10px; z-index: 9999;
    padding: 4px 8px; border-radius: var(--radius);
    background: rgb(0 0 0 / 0.7); color: #fff;
    font: 400 13px/1.4 ui-monospace, monospace; pointer-events: none;
  }
  .fps strong { font-size: 16px; }
  .fps span { display: block; color: #bbb; font-size: 11px; }
  :global(html.lite *) { filter: none !important; mix-blend-mode: normal !important; }
  :global(html.lite main) { background: var(--bg) !important; }
</style>
