// nooforge-ui/src/lib/tabs.ts
import { writable } from "svelte/store";

export type TabKey = "ingest" | "rag" | "search";

function init(): TabKey {
  const h = typeof window !== "undefined" ? window.location.hash.replace(/^#/, "") : "";
  return (h === "ingest" || h === "rag" || h === "search") ? (h as TabKey) : "ingest";
}

export const activeTab = writable<TabKey>(init());

if (typeof window !== "undefined") {
  window.addEventListener("hashchange", () => {
    const k = window.location.hash.replace(/^#/, "");
    if (k === "ingest" || k === "rag" || k === "search") activeTab.set(k as TabKey);
  });
}
