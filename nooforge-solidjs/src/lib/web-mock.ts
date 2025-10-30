// src/lib/web-mock.ts
/**
 * Mock API для разработки веб-версии без Tauri
 * Используется только когда запущено в браузере
 */

export async function mockInvokeBackend<T = string>(
  command: string,
  args?: Record<string, any>
): Promise<T> {
  // Симуляция задержки сети
  await new Promise(resolve => setTimeout(resolve, 500));

  switch (command) {
    case 'ingest_text': {
      const text = args?.text || '';
      return JSON.stringify({
        status: 'success',
        processed_chars: text.length,
        segments: Math.ceil(text.length / 100),
        message: `Текст обработан (${text.length} символов)`,
      }, null, 2) as T;
    }

    case 'ingest_file': {
      const path = args?.path || 'unknown.txt';
      return JSON.stringify({
        status: 'success',
        file: path,
        size_kb: 42,
        segments: 15,
        message: `Файл ${path} успешно обработан`,
      }, null, 2) as T;
    }

    case 'rag': {
      const query = args?.q || '';
      const limit = args?.limit || 5;
      
      const mockContext = [
        'Source: doc1.txt\nЭто первый фрагмент контекста из документа 1.',
        'Source: doc2.txt\nЭто второй фрагмент контекста из документа 2.',
        'Source: doc3.txt\nЭто третий фрагмент контекста из документа 3.',
      ].slice(0, limit).join('\n---\n\n');

      return JSON.stringify({
        answer: `Это тестовый ответ на запрос "${query}". В реальном приложении здесь будет ответ от LLM на основе найденного контекста.`,
        context: mockContext,
      }) as T;
    }

    case 'search': {
      const query = args?.q || '';
      const limit = args?.limit || 10;
      
      const results = Array.from({ length: Math.min(limit, 5) }, (_, i) => ({
        id: i + 1,
        score: 0.95 - i * 0.1,
        source: `document_${i + 1}.txt`,
        snippet: `Фрагмент текста содержащий "${query}"...`,
      }));

      return JSON.stringify({
        query,
        total: results.length,
        results,
      }, null, 2) as T;
    }

    default:
      throw new Error(`Unknown command: ${command}`);
  }
}

/**
 * Проверка доступности mock API
 */
export function isWebMockAvailable(): boolean {
  return typeof window !== 'undefined' && !('__TAURI__' in window);
}
