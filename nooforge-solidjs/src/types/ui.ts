// src/types/ui.ts
export type Status = "idle" | "loading" | "ok" | "error";

export type TabKey = "ingest" | "rag" | "search";

export interface ContextBlock {
  source: string;
  content: string;
}

export interface RagResponse {
  answer: string;
  contextBlocks: ContextBlock[];
}
