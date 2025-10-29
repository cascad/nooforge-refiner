<!-- nooforge-ui/src/components/StatusDot.svelte -->
<script lang="ts">
  export type Status = "idle" | "loading" | "ok" | "error";
  export let status: Status = "idle";
  export let title = "";
</script>

<span title={title}
  class="inline-block w-2.5 h-2.5 rounded-full"
  style="background:
    {status==='idle' ? '#6b7280' :    /* gray-500 */
     status==='loading' ? '#f59e0b' : /* amber-500 */
     status==='ok' ? '#10b981' :      /* emerald-500 */
     '#ef4444'}"                      /* red-500 */
></span>
