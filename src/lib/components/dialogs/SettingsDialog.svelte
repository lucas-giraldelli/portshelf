<script lang="ts">
  // App settings: appearance and language, and the ROM folder with each system's game files.
  import Dialog from "$lib/components/ui/Dialog.svelte";
  import SystemLogo from "$lib/components/media/SystemLogo.svelte";
  import { prefs, tr } from "$lib/prefs.svelte";
  import { setFullscreen } from "$lib/fullscreen";
  import { shelf } from "$lib/shelf.svelte";
  import { themes } from "$lib/themes";
  import { languages, type Key } from "$lib/i18n";

  let { onclose }: { onclose: () => void } = $props();
</script>

<Dialog {onclose} labelledby="settings-title" width={720}>
  <div class="body">
    <button class="close" onclick={onclose} aria-label={tr("common.close")}>×</button>
    <h2 id="settings-title">{tr("settings.title")}</h2>

    <section class="group">
      <h3 class="group-title">{tr("settings.appearance")}</h3>
      <div class="row">
        <label class="field">
          <span>{tr("header.theme")}</span>
          <select bind:value={prefs.theme}>
            {#each Object.entries(themes) as [id, theme] (id)}
              <option value={id}>{theme.name}</option>
            {/each}
          </select>
          <small>{tr(`theme.${prefs.theme}` as Key)}</small>
        </label>
        <div class="field">
          <span>{tr("settings.mode")}</span>
          <div class="segmented" role="radiogroup" aria-label={tr("settings.mode")}>
            <button role="radio" aria-checked={prefs.mode === "dark"} class:on={prefs.mode === "dark"} onclick={() => (prefs.mode = "dark")}>☾ {tr("settings.dark")}</button>
            <button role="radio" aria-checked={prefs.mode === "light"} class:on={prefs.mode === "light"} onclick={() => (prefs.mode = "light")}>☀ {tr("settings.light")}</button>
          </div>
        </div>
        <div class="field">
          <span>{tr("settings.display")}</span>
          <div class="segmented" role="radiogroup" aria-label={tr("settings.display")}>
            <button role="radio" aria-checked={!prefs.fullscreen} class:on={!prefs.fullscreen} onclick={() => setFullscreen(false)}>{tr("settings.windowed")}</button>
            <button role="radio" aria-checked={prefs.fullscreen} class:on={prefs.fullscreen} onclick={() => setFullscreen(true)}>{tr("settings.fullscreen")}</button>
          </div>
        </div>
        <label class="field">
          <span>{tr("header.language")}</span>
          <select bind:value={prefs.lang}>
            {#each Object.entries(languages) as [id, label] (id)}
              <option value={id}>{label}</option>
            {/each}
          </select>
        </label>
      </div>
    </section>

    <section class="group">
      <h3 class="group-title">{tr("settings.roms")}</h3>
      <div class="split">
        <div>
          <span class="label">{tr("settings.romFolder")}</span>
          <code>{shelf.library?.roms_dir || tr("settings.noFolder")}</code>
        </div>
        <button class="tool" onclick={() => shelf.chooseRomFolder()}>{shelf.library?.roms_dir ? tr("settings.changeFolder") : tr("header.chooseRomFolder")}</button>
      </div>
      {#if shelf.library?.roms_dir}
        <ul class="systems">
          {#each shelf.consoles as [id, info] (id)}
            {@const st = shelf.systemRomStatus(id)}
            <li class="system-row">
              <SystemLogo {id} name={info.name} height={22} />
              <span class="state" class:ok={st.installed > 0 && st.ready === st.installed} class:warn={st.ready < st.installed}>
                {st.installed ? tr("settings.readyOf", { ready: st.ready, installed: st.installed }) : tr("settings.noneInstalled")}
              </span>
              <button class="tool small" onclick={() => shelf.chooseSystemFile(id)}>{tr("settings.chooseFile")}</button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

  </div>
</Dialog>

<style>
  .body { padding: 24px 28px; }
  h2 { margin: 0 0 8px; }
  .group { border-top: 1px solid var(--border); padding-top: 14px; margin-top: 16px; }
  .group-title { margin: 0 0 12px; font-size: 13px; letter-spacing: 1.5px; text-transform: uppercase; color: var(--muted); }
  .row { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 14px; }
  small { color: var(--muted); font-size: 13px; }
  .split { display: flex; gap: 14px; align-items: center; justify-content: space-between; flex-wrap: wrap; }
  .split .label { display: block; font-size: 14px; color: var(--muted); }
  .split code { font-size: 14px; color: var(--text); word-break: break-all; }
  .systems { list-style: none; padding: 0; margin: 14px 0 0; display: grid; gap: 6px; }
  .system-row { display: grid; grid-template-columns: 170px 1fr auto; align-items: center; gap: 12px; padding: 8px 12px; border-radius: var(--radius); background: var(--surface); }
  .state { font-size: 14px; color: var(--muted); }
  .state.ok { color: var(--ok); }
  .state.warn { color: var(--warn); }
</style>
