<!-- nooforge-ui/src/components/Spinner.svelte -->
<script lang="ts">
  export let size = 16;
</script>

<svg
  width={size} height={size} viewBox="0 0 24 24"
  class="animate-spin"
  style="vertical-align: -2px"
>
  <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="3" fill="none" opacity="0.2"/>
  <path d="M22 12a10 10 0 0 0-10-10" stroke="currentColor" stroke-width="3" fill="none"/>
</svg>

<style>
  .animate-spin { animation: spin 0.8s linear infinite }
  @keyframes spin { to { transform: rotate(360deg) } }
</style>
