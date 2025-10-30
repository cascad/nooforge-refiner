// src/pages/Rag.tsx
import { Component, createSignal, For, Show } from 'solid-js';
import Spinner from '../components/Spinner';
import StatusDot from '../components/StatusDot';
import { invokeBackend, copyToClipboard } from '../lib/platform';
import { parseRagResponse, basename, formatError } from '../lib/utils';
import type { Status, ContextBlock } from '../types/ui';

const Rag: Component = () => {
  const [query, setQuery] = createSignal('');
  const [limit, setLimit] = createSignal(5);
  const [busy, setBusy] = createSignal(false);
  const [status, setStatus] = createSignal<Status>('idle');
  const [statusTitle, setStatusTitle] = createSignal('Готов');
  
  const [answer, setAnswer] = createSignal('');
  const [contextBlocks, setContextBlocks] = createSignal<ContextBlock[]>([]);
  const [openBlocks, setOpenBlocks] = createSignal<Set<number>>(new Set());
  
  const [ctxOpen, setCtxOpen] = createSignal(true);
  const [showDebug, setShowDebug] = createSignal(false);
  const [raw, setRaw] = createSignal('');

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey && !busy()) {
      e.preventDefault();
      run();
    }
  };

  const resetView = () => {
    setAnswer('');
    setContextBlocks([]);
    setOpenBlocks(new Set());
  };

  const handleCopy = async (text: string) => {
    const success = await copyToClipboard(text);
    if (success) {
      setStatus('ok');
      setStatusTitle('Скопировано');
      setTimeout(() => {
        setStatus('idle');
        setStatusTitle('Готов');
      }, 800);
    } else {
      setStatus('error');
      setStatusTitle('Не удалось скопировать');
      setTimeout(() => {
        setStatus('idle');
        setStatusTitle('Готов');
      }, 1200);
    }
  };

  const toggleBlock = (index: number) => {
    const current = new Set(openBlocks());
    if (current.has(index)) {
      current.delete(index);
    } else {
      current.add(index);
    }
    setOpenBlocks(current);
  };

  const expandAll = () => {
    setOpenBlocks(new Set(contextBlocks().map((_, i) => i)));
  };

  const collapseAll = () => {
    setOpenBlocks(new Set());
  };

  const run = async () => {
    const q = query().trim();
    if (!q) return;

    setBusy(true);
    setStatus('loading');
    setStatusTitle('Выполняется…');
    resetView();
    setRaw('');

    try {
      const result = await invokeBackend<string>('rag', { q, limit: limit() });
      setRaw(result || '');
      
      const parsed = parseRagResponse(result);
      setAnswer(parsed.answer || '(пусто)');
      setContextBlocks(parsed.contextBlocks);

      const isBad = !parsed.answer || /^error[:\s]/i.test(parsed.answer);
      setStatus(isBad ? 'error' : 'ok');
      setStatusTitle(isBad ? 'Ответ плохой / ошибка' : 'Успешно');
    } catch (error) {
      const errorMsg = formatError(error);
      setRaw(errorMsg);
      setAnswer(errorMsg);
      setStatus('error');
      setStatusTitle('Invoke error');
    } finally {
      setBusy(false);
    }
  };

  return (
    <section class="max-w-5xl mx-auto space-y-6">
      {/* Header */}
      <div class="flex items-center gap-3 flex-wrap">
        <h2 class="text-2xl font-bold tracking-tight bg-gradient-to-r from-white to-neutral-400 bg-clip-text text-transparent">
          RAG
        </h2>
        <Show when={busy()}>
          <Spinner size={16} />
        </Show>
        <StatusDot status={status()} title={statusTitle()} />
        
        <div class="ml-auto flex items-center gap-3 text-sm">
          <label class="inline-flex items-center gap-2 opacity-80">
            <span class="whitespace-nowrap text-neutral-400">Limit</span>
            <input
              class="w-20 bg-neutral-950/70 rounded-lg px-3 py-2 border border-white/10 focus:outline-none focus:ring-2 focus:ring-emerald-500/50"
              type="number"
              min="1"
              value={limit()}
              onInput={(e) => setLimit(parseInt(e.currentTarget.value) || 5)}
            />
          </label>
          <button
            class="px-3 py-1.5 rounded-lg bg-white/10 hover:bg-white/20 text-xs font-medium transition-all"
            onClick={() => setShowDebug(!showDebug())}
          >
            {showDebug() ? 'Debug ▲' : 'Debug ▼'}
          </button>
        </div>
      </div>

      {/* Input */}
      <div class="flex gap-3">
        <input
          class="flex-1 bg-neutral-950/70 rounded-xl px-5 py-3 border border-white/10 focus:outline-none focus:ring-2 focus:ring-emerald-500/50 focus:border-transparent placeholder:text-neutral-500 transition-all"
          placeholder="Вопрос… (Enter — отправить)"
          value={query()}
          onInput={(e) => setQuery(e.currentTarget.value)}
          onKeyDown={handleKeyDown}
        />
        <button
          class="px-6 py-3 rounded-xl bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 disabled:cursor-not-allowed font-medium transition-all shadow-lg hover:shadow-emerald-500/25 flex items-center gap-2"
          disabled={busy() || !query().trim()}
          onClick={run}
        >
          <Show when={busy()}>
            <Spinner size={14} />
          </Show>
          Ask
        </button>
      </div>

      {/* Answer Card */}
      <div class="rounded-2xl border border-white/10 bg-gradient-to-br from-neutral-900/90 to-neutral-900/70 backdrop-blur-sm shadow-2xl overflow-hidden">
        <div class="flex items-center justify-between px-5 py-3 border-b border-white/10 bg-neutral-900/50">
          <div class="text-xs uppercase tracking-wider text-neutral-400 font-semibold">Answer</div>
          <button
            class="text-xs px-3 py-1.5 rounded-lg bg-white/10 hover:bg-white/20 font-medium transition-all"
            onClick={() => handleCopy(answer())}
          >
            Копировать
          </button>
        </div>
        <div class="px-6 py-5 whitespace-pre-wrap leading-relaxed text-[15px]">
          <Show when={answer()} fallback={<span class="text-neutral-500">(пусто)</span>}>
            {answer()}
          </Show>
        </div>
      </div>

      {/* Context Card */}
      <div class="rounded-2xl border border-white/10 bg-gradient-to-br from-neutral-900/90 to-neutral-900/70 backdrop-blur-sm shadow-2xl overflow-hidden">
        <div class="flex items-center justify-between px-5 py-3 border-b border-white/10 bg-neutral-900/50">
          <div class="text-xs uppercase tracking-wider text-neutral-400 font-semibold">
            Context
            <Show when={contextBlocks().length > 0}>
              <span class="ml-1">({contextBlocks().length})</span>
            </Show>
          </div>
          <div class="flex items-center gap-2">
            <button
              class="text-xs px-3 py-1.5 rounded-lg bg-white/10 hover:bg-white/20 font-medium transition-all"
              onClick={() => setCtxOpen(!ctxOpen())}
            >
              {ctxOpen() ? 'Свернуть' : 'Развернуть'}
            </button>
            <Show when={ctxOpen() && contextBlocks().length > 1}>
              <button
                class="text-xs px-3 py-1.5 rounded-lg bg-white/10 hover:bg-white/20 font-medium transition-all"
                onClick={expandAll}
              >
                Развернуть всё
              </button>
              <button
                class="text-xs px-3 py-1.5 rounded-lg bg-white/10 hover:bg-white/20 font-medium transition-all"
                onClick={collapseAll}
              >
                Свернуть всё
              </button>
            </Show>
          </div>
        </div>

        <Show when={ctxOpen()}>
          <Show
            when={contextBlocks().length > 0}
            fallback={
              <div class="px-6 py-5 text-sm text-neutral-500">(нет контекста)</div>
            }
          >
            <div class="p-4 grid gap-3 md:grid-cols-2">
              <For each={contextBlocks()}>
                {(block, index) => (
                  <div class="rounded-xl border border-white/10 bg-black/30 overflow-hidden hover:border-white/20 transition-all">
                    <div class="px-4 py-3 border-b border-white/10 bg-neutral-900/30">
                      <div class="text-sm font-medium truncate">{basename(block.source)}</div>
                      <div class="text-[11px] text-neutral-500 truncate mt-0.5" title={block.source}>
                        {block.source}
                      </div>
                    </div>

                    <div class="px-4 py-2.5 flex items-center gap-2">
                      <button
                        class="text-xs px-2.5 py-1 rounded-lg bg-white/10 hover:bg-white/20 font-medium transition-all"
                        onClick={() => toggleBlock(index())}
                      >
                        {openBlocks().has(index()) ? 'Скрыть' : 'Показать'}
                      </button>
                      <button
                        class="text-xs px-2.5 py-1 rounded-lg bg-white/10 hover:bg-white/20 font-medium transition-all"
                        onClick={() => handleCopy(block.content)}
                      >
                        Копировать
                      </button>
                    </div>

                    <Show when={openBlocks().has(index())}>
                      <div class="px-4 pb-4">
                        <pre class="bg-black/40 rounded-lg p-3 text-xs whitespace-pre-wrap overflow-auto max-h-64 leading-relaxed border border-white/5">
                          {block.content}
                        </pre>
                      </div>
                    </Show>
                  </div>
                )}
              </For>
            </div>
          </Show>
        </Show>
      </div>

      {/* Debug Panel */}
      <Show when={showDebug()}>
        <div class="rounded-2xl border border-white/10 bg-gradient-to-br from-neutral-900/90 to-neutral-900/70 backdrop-blur-sm shadow-2xl overflow-hidden">
          <div class="px-5 py-3 border-b border-white/10 bg-neutral-900/50 text-xs uppercase tracking-wider text-neutral-400 font-semibold">
            RAW
          </div>
          <pre class="px-6 py-5 text-xs whitespace-pre-wrap font-mono leading-relaxed overflow-x-auto">
            <Show when={raw()} fallback={<span class="text-neutral-500">(пусто)</span>}>
              {raw()}
            </Show>
          </pre>
        </div>
      </Show>
    </section>
  );
};

export default Rag;
