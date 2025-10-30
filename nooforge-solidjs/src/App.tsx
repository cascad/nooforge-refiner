// src/App.tsx
import { Component, createSignal, Switch, Match, onMount, onCleanup } from 'solid-js';
import Ingest from './pages/Ingest';
import Rag from './pages/Rag';
import Search from './pages/Search';
import type { TabKey } from './types/ui';

const App: Component = () => {
  // Initialize from URL hash
  const getInitialTab = (): TabKey => {
    if (typeof window === 'undefined') return 'ingest';
    const hash = window.location.hash.replace(/^#/, '');
    return (hash === 'ingest' || hash === 'rag' || hash === 'search') ? hash : 'ingest';
  };

  const [activeTab, setActiveTab] = createSignal<TabKey>(getInitialTab());

  const switchTab = (tab: TabKey) => {
    setActiveTab(tab);
    if (typeof window !== 'undefined') {
      window.location.hash = tab;
    }
  };

  const handleHashChange = () => {
    const hash = window.location.hash.replace(/^#/, '');
    if (hash === 'ingest' || hash === 'rag' || hash === 'search') {
      setActiveTab(hash);
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    const isMac = navigator.platform.includes('Mac');
    const modKey = isMac ? e.metaKey : e.ctrlKey;
    
    if (!modKey) return;
    
    if (e.key === '1') {
      e.preventDefault();
      switchTab('ingest');
    } else if (e.key === '2') {
      e.preventDefault();
      switchTab('rag');
    } else if (e.key === '3') {
      e.preventDefault();
      switchTab('search');
    }
  };

  onMount(() => {
    window.addEventListener('hashchange', handleHashChange);
    window.addEventListener('keydown', handleKeyDown);
  });

  onCleanup(() => {
    window.removeEventListener('hashchange', handleHashChange);
    window.removeEventListener('keydown', handleKeyDown);
  });

  const tabClasses = (tab: TabKey) => {
    const isActive = activeTab() === tab;
    return `px-5 py-2.5 rounded-xl font-medium transition-all ${
      isActive
        ? 'bg-white/10 border border-white/20 shadow-lg'
        : 'hover:bg-white/5 border border-transparent hover:border-white/10'
    }`;
  };

  return (
    <div class="min-h-screen bg-gradient-to-br from-neutral-950 via-neutral-900 to-neutral-950 text-neutral-100">
      <div class="max-w-7xl mx-auto px-4 py-6">
        {/* Header with tabs */}
        <header class="mb-8">
          <div class="flex items-center justify-between mb-6">
            <div>
              <h1 class="text-3xl font-bold tracking-tight bg-gradient-to-r from-white via-neutral-200 to-neutral-400 bg-clip-text text-transparent">
                NooForge Refiner
              </h1>
              <p class="text-sm text-neutral-500 mt-1">
                Semantic processing and enrichment pipeline
              </p>
            </div>
            <div class="text-xs text-neutral-600">
              <kbd class="px-2 py-1 rounded bg-white/5 border border-white/10 font-mono">
                Ctrl/⌘ + 1-3
              </kbd>
              <span class="ml-2">для переключения вкладок</span>
            </div>
          </div>

          <nav class="flex items-center gap-2 border-b border-white/10 pb-4">
            <button
              class={tabClasses('ingest')}
              onClick={() => switchTab('ingest')}
            >
              Ingest
            </button>
            <button
              class={tabClasses('rag')}
              onClick={() => switchTab('rag')}
            >
              RAG
            </button>
            <button
              class={tabClasses('search')}
              onClick={() => switchTab('search')}
            >
              Search
            </button>
          </nav>
        </header>

        {/* Content */}
        <main>
          <Switch>
            <Match when={activeTab() === 'ingest'}>
              <Ingest />
            </Match>
            <Match when={activeTab() === 'rag'}>
              <Rag />
            </Match>
            <Match when={activeTab() === 'search'}>
              <Search />
            </Match>
          </Switch>
        </main>

        {/* Footer */}
        <footer class="mt-12 pt-6 border-t border-white/5 text-center text-xs text-neutral-600">
          <p>
            NooForge Refiner — semantic segmentation and enrichment through LLM
          </p>
        </footer>
      </div>
    </div>
  );
};

export default App;
