<!-- nooforge-ui/src/App.svelte -->
<script lang="ts">
  import Ingest from "./pages/Ingest.svelte";
  import Rag from "./pages/Rag.svelte";
  import Search from "./pages/Search.svelte";
  import { activeTab, type TabKey } from "./lib/tabs";
  let tab: TabKey; $: tab = $activeTab;

  function setTab(k: TabKey) {
    if (typeof window !== "undefined") window.location.hash = k;
    activeTab.set(k);
  }

  function onWindowKey(e: KeyboardEvent) {
    const mac = navigator.platform.includes("Mac");
    const mod = mac ? e.metaKey : e.ctrlKey;
    if (!mod) return;
    if (e.key === "1") setTab("ingest");
    if (e.key === "2") setTab("rag");
    if (e.key === "3") setTab("search");
  }
</script>

<svelte:window on:keydown={onWindowKey} />

<div class="min-h-screen bg-neutral-950 text-neutral-100">
  <div class="max-w-6xl mx-auto p-4">
    <div class="flex items-center gap-2 border-b border-white/10 pb-3">
      <button class="px-4 py-2 rounded-xl transition {tab==='ingest' ? 'bg-white/10 border border-white/20' : 'hover:bg-white/5 border border-transparent'}" on:click={() => setTab('ingest')}>Ingest</button>
      <button class="px-4 py-2 rounded-xl transition {tab==='rag' ? 'bg-white/10 border border-white/20' : 'hover:bg-white/5 border border-transparent'}" on:click={() => setTab('rag')}>RAG</button>
      <button class="px-4 py-2 rounded-xl transition {tab==='search' ? 'bg-white/10 border border-white/20' : 'hover:bg-white/5 border border-transparent'}" on:click={() => setTab('search')}>Search</button>
    </div>

    <div class="mt-6">
      {#if tab === "ingest"} <Ingest />
      {:else if tab === "rag"} <Rag />
      {:else} <Search /> {/if}
    </div>
  </div>
</div>
