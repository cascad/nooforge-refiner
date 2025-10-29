<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Spinner from "../components/Spinner.svelte";
  import type { Status } from "../types/ui";

  let text = "";
  let busyText = false;
  let busyFile = false;

  let statusText: Status = "idle";
  let statusFile: Status = "idle";
  let titleText = "Готов";
  let titleFile = "Готов";

  let resultText = "";
  let resultFile = "";

  function onKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      runText();
    }
  }

  function setTextStatus(k: Status, t: string) { statusText = k; titleText = t; }
  function setFileStatus(k: Status, t: string) { statusFile = k; titleFile = t; }

  async function runText() {
    if (!text.trim() || busyText) return;
    busyText = true;
    setTextStatus("loading", "Выполняется…");
    resultText = "";

    try {
      const res = await invoke<string>("ingest_text", { text });
      resultText = res ?? "";
      setTextStatus("ok", "Успешно");
    } catch (e) {
      resultText = typeof e === "string" ? e : JSON.stringify(e, Object.getOwnPropertyNames(e as object), 2);
      setTextStatus("error", "Ошибка");
    } finally {
      busyText = false;
    }
  }

  async function onFilePick(ev: Event) {
    const input = ev.target as HTMLInputElement;
    if (!input.files || input.files.length === 0) return;
    await runFile(input.files[0]);
    input.value = "";
  }

  async function onDrop(ev: DragEvent) {
    ev.preventDefault();
    if (!ev.dataTransfer || ev.dataTransfer.files.length === 0) return;
    await runFile(ev.dataTransfer.files[0]);
  }
  function onDragOver(ev: DragEvent) { ev.preventDefault(); }

  async function runFile(file: File) {
    if (busyFile) return;
    // В Tauri у File есть .path (в Web может не быть), но тут десктоп — берём as any
    const anyFile = file as any;
    const path: string | undefined = anyFile?.path;
    if (!path) {
      setFileStatus("error", "Нет пути к файлу");
      return;
    }

    busyFile = true;
    setFileStatus("loading", "Загрузка…");
    resultFile = "";

    try {
      const res = await invoke<string>("ingest_file", { path });
      resultFile = res ?? "";
      setFileStatus("ok", "Успешно");
    } catch (e) {
      resultFile = typeof e === "string" ? e : JSON.stringify(e, Object.getOwnPropertyNames(e as object), 2);
      setFileStatus("error", "Ошибка");
    } finally {
      busyFile = false;
    }
  }
</script>

<section class="space-y-8">

  <div>
    <div class="flex items-center gap-2">
      <h2 class="text-xl font-semibold">Ingest (Text)</h2>
      {#if busyText}<Spinner size={16} />{/if}
      <status status={statusText} title={titleText} />
    </div>

    <div class="mt-3">
      <textarea
        class="w-full min-h-[140px] bg-neutral-900 rounded-lg px-3 py-2 border border-white/10"
        placeholder="Текст для ингеста… (Ctrl+Enter — отправить)"
        bind:value={text}
        on:keydown={onKeyDown}
      />
      <div class="mt-2 flex justify-end">
        <button class="px-4 py-2 rounded-xl bg-emerald-600 disabled:opacity-50" disabled={busyText || !text.trim()} on:click={runText}>
          {#if busyText}<Spinner size={14} /> {/if} Ingest text
        </button>
      </div>
    </div>

    {#if resultText}
      <pre class="mt-3 bg-neutral-900 rounded-lg p-3 border border-white/10 whitespace-pre-wrap text-sm">{resultText}</pre>
    {/if}
  </div>

  <div>
    <div class="flex items-center gap-2">
      <h2 class="text-xl font-semibold">Ingest (File)</h2>
      {#if busyFile}<Spinner size={16} />{/if}
      <status status={statusFile} title={titleFile} />
    </div>

    <div
      class="mt-3 border-2 border-dashed border-white/20 rounded-xl p-6 text-center bg-neutral-900"
      on:drop={onDrop}
      on:dragover={onDragOver}
    >
      Перетащи файл сюда
      <div class="mt-3">
        <input type="file" class="hidden" id="ingest-file" on:change={onFilePick} />
        <label for="ingest-file" class="px-4 py-2 rounded-xl bg-white/10 hover:bg-white/20 cursor-pointer select-none">
          Или выбери файл…
        </label>
      </div>
    </div>

    {#if resultFile}
      <pre class="mt-3 bg-neutral-900 rounded-lg p-3 border border-white/10 whitespace-pre-wrap text-sm">{resultFile}</pre>
    {/if}
  </div>

</section>
