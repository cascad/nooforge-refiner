<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Spinner from "../components/Spinner.svelte";
  import type { Status } from "../types/ui";

  let query = "";
  let limit: number = 10;

  let busy = false;
  let status: Status = "idle";
  let statusTitle = "Готов";
  let result = "";

  function setStatus(kind: Status, title: string) {
    status = kind;
    statusTitle = title;
  }

  function onKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      run();
    }
  }

  async function run() {
    if (!query.trim() || busy) return;
    busy = true;
    setStatus("loading", "Выполняется…");
    result = "";

    try {
      const res = await invoke<string>("search", { q: query, limit });
      result = res ?? "";
      setStatus("ok", "Успешно");
    } catch (e) {
      result = typeof e === "string" ? e : JSON.stringify(e, Object.getOwnPropertyNames(e as object), 2);
      setStatus("error", "Ошибка");
    } finally {
      busy = false;
    }
  }
</script>

<section>
  <div class="flex items-center gap-2">
    <h2 class="text-xl font-semibold">Search</h2>
    {#if busy}<Spinner size={16} />{/if}
    <status status={status} title={statusTitle} />
  </div>

  <div class="mt-4 flex gap-2">
    <input
      class="flex-1 bg-neutral-900 rounded-lg px-3 py-2 border border-white/10"
      placeholder="Запрос… (Enter — отправить)"
      bind:value={query}
      on:keydown={onKeyDown}
    />
    <input
      class="w-24 bg-neutral-900 rounded-lg px-3 py-2 border border-white/10"
      type="number" min="1" bind:value={limit} />
    <button class="px-4 py-2 rounded-xl bg-emerald-600 disabled:opacity-50" disabled={busy || !query.trim()} on:click={run}>
      {#if busy}<Spinner size={14} /> {/if} Go
    </button>
  </div>

  {#if result}
    <pre class="mt-4 bg-neutral-900 rounded-lg p-3 border border-white/10 whitespace-pre-wrap text-sm">{result}</pre>
  {/if}
</section>
