// src/pages/Ingest.tsx
import { Component, createSignal, Show, onMount, onCleanup } from 'solid-js';
import Spinner from '../components/Spinner';
import StatusDot from '../components/StatusDot';
import { invokeBackend, selectFile, setupTauriFileDrop } from '../lib/platform';
import { formatError } from '../lib/utils';
import type { Status } from '../types/ui';

const Ingest: Component = () => {
  // Text ingest state
  const [text, setText] = createSignal('');
  const [busyText, setBusyText] = createSignal(false);
  const [statusText, setStatusText] = createSignal<Status>('idle');
  const [titleText, setTitleText] = createSignal('Готов');
  const [resultText, setResultText] = createSignal('');

  // File ingest state
  const [busyFile, setBusyFile] = createSignal(false);
  const [statusFile, setStatusFile] = createSignal<Status>('idle');
  const [titleFile, setTitleFile] = createSignal('Готов');
  const [resultFile, setResultFile] = createSignal('');

  let dropZoneRef: HTMLDivElement | undefined;

  const handleTextKeyDown = (e: KeyboardEvent) => {
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
      e.preventDefault();
      runText();
    }
  };

  const runText = async () => {
    const textValue = text().trim();
    if (!textValue || busyText()) return;

    setBusyText(true);
    setStatusText('loading');
    setTitleText('Выполняется…');
    setResultText('');

    try {
      const result = await invokeBackend<string>('ingest_text', { text: textValue });
      setResultText(result || '');
      setStatusText('ok');
      setTitleText('Успешно');
    } catch (error) {
      setResultText(formatError(error));
      setStatusText('error');
      setTitleText('Ошибка');
    } finally {
      setBusyText(false);
    }
  };

  const pickFile = async () => {
    try {
      const path = await selectFile();
      if (path) {
        await runFileByPath(path);
      }
    } catch (error) {
      setResultFile(formatError(error));
      setStatusFile('error');
      setTitleFile('Диалог не открылся');
    }
  };

  const runFileByPath = async (path: string) => {
    setBusyFile(true);
    setStatusFile('loading');
    setTitleFile('Загрузка…');
    setResultFile('');

    try {
      const result = await invokeBackend<string>('ingest_file', { path });
      setResultFile(result || '');
      setStatusFile('ok');
      setTitleFile('Успешно');
    } catch (error) {
      setResultFile(formatError(error));
      setStatusFile('error');
      setTitleFile('Ошибка');
    } finally {
      setBusyFile(false);
    }
  };

  let unlistenFileDrop: (() => void) | undefined;

  // Биндим Tauri file-drop event
  onMount(async () => {
    console.log('Ingest mounted, setting up Tauri file drop');
    
    unlistenFileDrop = await setupTauriFileDrop((paths) => {
      console.log('Files dropped:', paths);
      if (paths.length > 0 && !busyFile()) {
        runFileByPath(paths[0]);
      }
    });
  });

  onCleanup(() => {
    console.log('Cleaning up file drop listener');
    if (unlistenFileDrop) {
      unlistenFileDrop();
    }
  });

  return (
    <div class="space-y-6">
      {/* Text Ingest Section */}
      <div class="rounded-2xl border border-white/10 bg-gradient-to-br from-neutral-900/90 to-neutral-900/70 backdrop-blur-sm p-6 shadow-2xl">
        <div class="flex items-center gap-3 mb-4">
          <h3 class="text-lg font-semibold tracking-tight">Ingest (Text)</h3>
          <Show when={busyText()}>
            <Spinner size={16} />
          </Show>
          <StatusDot status={statusText()} title={titleText()} />
        </div>

        <textarea
          class="w-full h-32 bg-neutral-950/50 rounded-xl px-4 py-3 border border-white/10 focus:outline-none focus:ring-2 focus:ring-emerald-500/50 focus:border-transparent resize-none transition-all placeholder:text-neutral-500"
          placeholder="Введите текст для обработки… (Ctrl+Enter для отправки)"
          value={text()}
          onInput={(e) => setText(e.currentTarget.value)}
          onKeyDown={handleTextKeyDown}
        />

        <button
          class="mt-4 px-6 py-2.5 rounded-xl bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 disabled:cursor-not-allowed font-medium transition-all shadow-lg hover:shadow-emerald-500/25"
          disabled={busyText() || !text().trim()}
          onClick={runText}
        >
          <Show when={busyText()} fallback="Обработать">
            <span class="flex items-center gap-2">
              <Spinner size={14} /> Обработка…
            </span>
          </Show>
        </button>

        <Show when={resultText()}>
          <pre class="mt-4 bg-neutral-950/70 rounded-xl p-4 border border-white/10 whitespace-pre-wrap text-sm leading-relaxed overflow-x-auto">
            {resultText()}
          </pre>
        </Show>
      </div>

      {/* File Ingest Section */}
      <div class="rounded-2xl border border-white/10 bg-gradient-to-br from-neutral-900/90 to-neutral-900/70 backdrop-blur-sm p-6 shadow-2xl">
        <div class="flex items-center gap-3 mb-4">
          <h3 class="text-lg font-semibold tracking-tight">Ingest (File)</h3>
          <Show when={busyFile()}>
            <Spinner size={16} />
          </Show>
          <StatusDot status={statusFile()} title={titleFile()} />
        </div>

        <div
          ref={dropZoneRef}
          class="rounded-xl border-2 border-dashed border-white/20 bg-neutral-950/30 p-12 text-center hover:border-white/40 hover:bg-neutral-950/50 transition-all cursor-pointer group"
        >
          <div class="text-neutral-400 group-hover:text-neutral-300 transition-colors mb-3">
            <svg class="w-12 h-12 mx-auto mb-3 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
            </svg>
            Перетащите файл сюда
          </div>
          <button
            class="px-5 py-2.5 rounded-xl bg-white/10 hover:bg-white/20 font-medium transition-all disabled:opacity-50"
            disabled={busyFile()}
            onClick={pickFile}
          >
            Или выберите файл…
          </button>
        </div>

        <Show when={resultFile()}>
          <pre class="mt-4 bg-neutral-950/70 rounded-xl p-4 border border-white/10 whitespace-pre-wrap text-sm leading-relaxed overflow-x-auto">
            {resultFile()}
          </pre>
        </Show>
      </div>
    </div>
  );
};

export default Ingest;
