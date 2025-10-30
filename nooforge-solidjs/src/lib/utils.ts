// src/lib/utils.ts
import type { RagResponse, ContextBlock } from '../types/ui';

/**
 * Извлечь имя файла из пути
 */
export function basename(path: string): string {
  const s = path.replace(/[/\\]+$/, '');
  const i = Math.max(s.lastIndexOf('/'), s.lastIndexOf('\\'));
  return i >= 0 ? s.slice(i + 1) : s;
}

/**
 * Парсинг ответа RAG из JSON
 */
export function parseRagResponse(rawResponse: string): RagResponse {
  try {
    const obj = JSON.parse(rawResponse);
    const answer = String(obj.answer ?? obj.output ?? obj.text ?? '');
    const context = String(obj.context ?? '');
    
    return {
      answer: answer || rawResponse,
      contextBlocks: extractContextBlocks(context),
    };
  } catch {
    return {
      answer: rawResponse,
      contextBlocks: [],
    };
  }
}

/**
 * Извлечение блоков контекста из строки
 */
function extractContextBlocks(contextString: string): ContextBlock[] {
  if (!contextString?.trim()) return [];
  
  const parts = contextString
    .split(/\n-{3,}\n+/g)
    .map(p => p.trim())
    .filter(Boolean);
  
  const blocks: ContextBlock[] = [];
  
  for (const part of parts) {
    const match = part.match(/^Source:\s*(.+?)\s*\r?\n/);
    if (match) {
      const source = match[1].trim();
      const content = part.slice(match[0].length).trim();
      blocks.push({ source, content });
    } else {
      blocks.push({ source: '(unnamed source)', content: part });
    }
  }
  
  return blocks;
}

/**
 * Форматирование ошибок
 */
export function formatError(error: unknown): string {
  if (typeof error === 'string') return error;
  if (error instanceof Error) return error.message;
  try {
    return JSON.stringify(error, Object.getOwnPropertyNames(error as object), 2);
  } catch {
    return String(error);
  }
}
