// nooforge-solidjs/src/pages/Ingest.tsx
import { Component, createSignal, Show, onMount, onCleanup } from 'solid-js';
import Spinner from '../components/Spinner';
import StatusDot from '../components/StatusDot';
import DropZone from '../components/DropZone';
import { invokeBackend, selectFile, setupTauriFileDrop } from '../lib/platform';
import { normalizeDropPath } from '../lib/path_to_file';
import type { Status } from '../types/ui';

function canonPath(raw: string): string | null {
  let s = normalizeDropPath(raw);
  if (!s) return null;

  // JSON вида [{"resource":{fsPath:"d:\\..."}}, ...] или ["d:\\..."]
  if (s.trim().startsWith('[')) {
    try {
      const parsed = JSON.parse(s);
      const a = Array.isArray(parsed) ? parsed[0] : null;
      const fsPath = (a && typeof a === 'object') ? (a as any)?.resource?.fsPath : undefined;
      if (typeof fsPath === 'string') s = fsPath;
      else if (typeof a === 'string') s = a;
    } catch { /* ignore */ }
  }

  // file:///d%3A/... → D:/... ; /d:/... → D:/...
  if (s.startsWith('file://')) {
    s = s.replace(/^file:\/+/, '');
    try { s = decodeURI(s); } catch {}
    if (s.startsWith('/')) s = s.slice(1);
  }
  if (/^\/[A-Za-z]:\//.test(s)) s = s.slice(1);

  s = s.replace(/\\/g, '/');
  s = s.replace(/^([a-zA-Z]):\//, (_, d: string) => d.toUpperCase() + ':/');

  if (/^[A-Z]:\//.test(s) || /^\/\/[^/]+\/[^/]+/.test(s) || s.startsWith('/mnt/')) {
    return s;
  }
  return null;
}

const Ingest: Component = () => {
  // TEXT
  const [text, setText] = createSignal('');
  const [busyText, setBusyText] = createSignal(false);
  const [statusText, setStatusText] = createSignal<Status>('idle');
  const [titleText, setTitleText] = createSignal('Готов');
  const [resultText, setResultText] = createSignal('');

  // FILE
  const [busyFile, setBusyFile] = createSignal(false);
  const [statusFile, setStatusFile] = createSignal<Status>('idle');
  const [titleFile, setTitleFile] = createSignal('Готов');
  const [resultFile, setResultFile] = createSignal('');

  // ---- batching & dedup for a single drop ----
  let dropBatch = new Set<string>();
  let dropTimer: number | undefined;

  async function flushDropBatch() {
    const list = Array.from(dropBatch);
    dropBatch.clear(); dropTimer = undefined;

    const normalized = Array.from(
      new Set(
        list.map(canonPath).filter((x): x is string => !!x)
      )
    );

    if (!normalized.length) {
      console.debug('[Ingest] drop batch: nothing usable', list);
      return;
    }

    const path = normalized[0];
    console.debug('[Ingest] drop final path:', path);
    await runFileOnce(path);
  }

  function queuePaths(raws: string[]) {
    for (const r of raws) {
      if (!r || r === 'about:blank#blocked') continue;
      dropBatch.add(r);
    }
    if (dropTimer === undefined) {
      dropTimer = window.setTimeout(flushDropBatch, 50) as unknown as number;
    }
  }

  // ---------- TEXT ----------
  const handleTextKeyDown = (e: KeyboardEvent) => {
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
      e.preventDefault();
      void runText();
    }
  };

  const runText = async () => {
    const value = text().trim();
    if (!value || busyText()) return;

    setBusyText(true);
    setStatusText('loading');
    setTitleText('Выполняется…');
    setResultText('');
    try {
      const res = await invokeBackend<string>('ingest_text', { text: value });
      setResultText(res || '');
      setStatusText('ok');
      setTitleText('Успешно');
    } catch (err: any) {
      setResultText(String(err?.message ?? err));
      setStatusText('error');
      setTitleText('Ошибка');
    } finally {
      setBusyText(false);
    }
  };

  // ---------- FILE ----------
  async function runFileOnce(path: string) {
    if (!path || busyFile()) return;

    setBusyFile(true);
    setStatusFile('loading');
    setTitleFile('Загрузка…');
    setResultFile('');
    try {
      const res = await invokeBackend<string>('ingest_file', { path });
      setResultFile(res || '');
      setStatusFile('ok');
      setTitleFile('Успешно');
    } catch (err: any) {
      console.error('[Ingest] ingest_file error:', err);
      setResultFile(String(err?.message ?? err));
      setStatusFile('error');
      setTitleFile('Ошибка');
    } finally {
      setBusyFile(false);
    }
  }

  const pickFile = async () => {
    const path = await selectFile();
    if (path) queuePaths([path]);
  };

  // ---------- Native DnD (Explorer / VSCode Explorer) + Paste ----------
  onMount(async () => {
    // 1) Нативный таури-дроп (Проводник/VSCode Explorer)
    const unlisten = await setupTauriFileDrop(async (paths) => {
      if (!paths?.length) return;
      queuePaths(paths);
    });

    // 2) Ctrl+V путей
    const onPaste = async (e: ClipboardEvent) => {
      const t = e.clipboardData?.getData('text/plain')?.trim();
      if (!t) return;
      const lines = t.split(/\r?\n/).map(s => s.trim()).filter(Boolean);
      queuePaths(lines);
    };
    window.addEventListener('paste', onPaste);

    onCleanup(() => {
      try { unlisten(); } catch {}
      window.removeEventListener('paste', onPaste);
      if (dropTimer !== undefined) clearTimeout(dropTimer);
    });
  });

  return (
    <div class="space-y-6 select-auto pointer-events-auto">
      {/* TEXT */}
      <div class="rounded-2xl border border-white/10 bg-gradient-to-br from-neutral-900/90 to-neutral-900/70 backdrop-blur-sm p-6 shadow-2xl">
        <div class="flex items-center gap-3 mb-4">
          <h3 class="text-lg font-semibold tracking-tight">Ingest (Text)</h3>
          <Show when={busyText()}><Spinner size={16} /></Show>
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
            <span class="flex items-center gap-2"><Spinner size={14} /> Обработка…</span>
          </Show>
        </button>

        <Show when={resultText()}>
          <pre class="mt-4 bg-neutral-950/70 rounded-xl p-4 border border-white/10 whitespace-pre-wrap text-sm leading-relaxed overflow-x-auto">
            {resultText()}
          </pre>
        </Show>
      </div>

      {/* FILE */}
      <div class="rounded-2xl border border-white/10 bg-gradient-to-br from-neutral-900/90 to-neutral-900/70 backdrop-blur-sm p-6 shadow-2xl">
        <div class="flex items-center gap-3 mb-4">
          <h3 class="text-lg font-semibold tracking-tight">Ingest (File)</h3>
          <Show when={busyFile()}><Spinner size={16} /></Show>
          <StatusDot status={statusFile()} title={titleFile()} />
        </div>

        <DropZone
          onPaths={async (paths) => {
            if (!paths?.length) return;
            // все варианты в батч (де-дуп, 50мс)
            for (const p of paths) queuePaths([p]);
          }}
        />

        <div class="mt-4">
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
