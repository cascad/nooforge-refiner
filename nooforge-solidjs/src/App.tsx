// nooforge-solidjs/src/App.tsx
import { createSignal, onMount, Component, Show } from "solid-js";
import Ingest from "./pages/Ingest";
import Rag from "./pages/Rag";
import Search from "./pages/Search";

const App: Component = () => {
  const [tab, setTab] = createSignal<"ingest" | "rag" | "search">("ingest");
  let headerRef: HTMLDivElement | undefined;

  onMount(() => {
    // Убираем drag-region у body
    document.body.removeAttribute("data-tauri-drag-region");
    
    // Настраиваем drag только для header
    const all = document.querySelectorAll<HTMLElement>("[data-tauri-drag-region]");
    all.forEach((el) => {
      if (headerRef && el === headerRef) {
        el.setAttribute("data-tauri-drag-region", "drag");
      } else {
        el.removeAttribute("data-tauri-drag-region");
      }
    });
  });

  const TabBtn: Component<{ id: "ingest" | "rag" | "search"; label: string }> = (p) => (
    <button
      onClick={() => setTab(p.id)}
      class={
        "px-4 py-1.5 rounded-md text-sm transition-colors " +
        (tab() === p.id ? "bg-white/10 text-white" : "text-neutral-400 hover:text-white hover:bg-white/5")
      }
    >
      {p.label}
    </button>
  );

  return (
    <div class="min-h-screen bg-neutral-900 text-neutral-100">
      {/* Drag region только для header */}
      <header
        ref={headerRef}
        data-tauri-drag-region="drag"
        class="h-11 px-4 flex items-center gap-2 border-b border-white/10 select-none cursor-move"
      >
        <TabBtn id="ingest" label="Ingest" />
        <TabBtn id="rag" label="RAG" />
        <TabBtn id="search" label="Search" />
      </header>

      {/* Main без каких-либо drag-region атрибутов */}
      <main class="max-w-5xl mx-auto py-6 px-4">
        <Show when={tab() === "ingest"}>
          <Ingest />
        </Show>
        <Show when={tab() === "rag"}>
          <Rag />
        </Show>
        <Show when={tab() === "search"}>
          <Search />
        </Show>
      </main>
    </div>
  );
};

export default App;