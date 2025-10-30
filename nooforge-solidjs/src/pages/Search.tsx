// src/pages/Search.tsx
import { Component, createSignal, Show } from 'solid-js';
import Spinner from '../components/Spinner';
import StatusDot from '../components/StatusDot';
import { invokeBackend } from '../lib/platform';
import { formatError } from '../lib/utils';
import type { Status } from '../types/ui';

const Search: Component = () => {
  const [query, setQuery] = createSignal('');
  const [limit, setLimit] = createSignal(10);
  const [busy, setBusy] = createSignal(false);
  const [status, setStatus] = createSignal<Status>('idle');
  const [statusTitle, setStatusTitle] = createSignal('Готов');
  const [result, setResult] = createSignal('');

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      run();
    }
  };

  const run = async () => {
    const q = query().trim();
    if (!q || busy()) return;

    setBusy(true);
    setStatus('loading');
    setStatusTitle('Выполняется…');
    setResult('');

    try {
      const res = await invokeBackend<string>('search', { q, limit: limit() });
      setResult(res || '');
      setStatus('ok');
      setStatusTitle('Успешно');
    } catch (error) {
      setResult(formatError(error));
      setStatus('error');
      setStatusTitle('Ошибка');
    } finally {
      setBusy(false);
    }
  };

  return (
    <section class="max-w-5xl mx-auto space-y-6">
      {/* Header */}
      <div class="flex items-center gap-3">
        <h2 class="text-2xl font-bold tracking-tight bg-gradient-to-r from-white to-neutral-400 bg-clip-text text-transparent">
          Search
        </h2>
        <Show when={busy()}>
          <Spinner size={16} />
        </Show>
        <StatusDot status={status()} title={statusTitle()} />
      </div>

      {/* Input */}
      <div class="flex gap-3 items-center">
        <input
          class="flex-1 bg-neutral-950/70 rounded-xl px-5 py-3 border border-white/10 focus:outline-none focus:ring-2 focus:ring-emerald-500/50 focus:border-transparent placeholder:text-neutral-500 transition-all"
          placeholder="Запрос… (Enter — отправить)"
          value={query()}
          onInput={(e) => setQuery(e.currentTarget.value)}
          onKeyDown={handleKeyDown}
        />
        <input
          class="w-24 bg-neutral-950/70 rounded-xl px-4 py-3 border border-white/10 focus:outline-none focus:ring-2 focus:ring-emerald-500/50 text-center"
          type="number"
          min="1"
          value={limit()}
          onInput={(e) => setLimit(parseInt(e.currentTarget.value) || 10)}
          title="Количество результатов"
        />
        <button
          class="px-6 py-3 rounded-xl bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 disabled:cursor-not-allowed font-medium transition-all shadow-lg hover:shadow-emerald-500/25 flex items-center gap-2"
          disabled={busy() || !query().trim()}
          onClick={run}
        >
          <Show when={busy()}>
            <Spinner size={14} />
          </Show>
          Go
        </button>
      </div>

      {/* Result Card */}
      <Show when={result()}>
        <div class="rounded-2xl border border-white/10 bg-gradient-to-br from-neutral-900/90 to-neutral-900/70 backdrop-blur-sm shadow-2xl overflow-hidden">
          <div class="px-5 py-3 border-b border-white/10 bg-neutral-900/50">
            <div class="text-xs uppercase tracking-wider text-neutral-400 font-semibold">
              Results
            </div>
          </div>
          <pre class="px-6 py-5 whitespace-pre-wrap text-sm leading-relaxed overflow-x-auto font-mono">
            {result()}
          </pre>
        </div>
      </Show>
    </section>
  );
};

export default Search;
