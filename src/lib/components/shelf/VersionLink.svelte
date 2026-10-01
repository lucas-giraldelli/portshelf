<script lang="ts">
  // PortShelf's version in the bottom right corner, linking to its release notes. Mouse only:
  // it is left out of keyboard and controller navigation.
  import { getVersion } from "@tauri-apps/api/app";
  import { openUrl } from "@tauri-apps/plugin-opener";

  let version = $state("");
  getVersion().then((v) => (version = v));
</script>

{#if version}
  <button class="version" tabindex="-1" onclick={() => openUrl(`https://github.com/lucas-giraldelli/portshelf/releases/tag/v${version}`)}>v{version}</button>
{/if}

<style>
  .version {
    position: absolute; right: 16px; bottom: 10px;
    background: none; border: 0; padding: 2px 4px; cursor: pointer;
    font: 400 13px var(--font-text); color: var(--muted); opacity: 0.8;
  }
  .version:hover { color: var(--accent); opacity: 1; text-decoration: underline; }
</style>
