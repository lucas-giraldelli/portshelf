<script lang="ts">
  import "../app.css";
  import FpsCounter from "$lib/components/ui/FpsCounter.svelte";
  let { children } = $props();
</script>

{@render children()}
<FpsCounter />
