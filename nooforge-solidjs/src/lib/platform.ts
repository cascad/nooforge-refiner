// src/lib/platform.ts
// Абстракция для работы с файлами и диалогами в вебе и Tauri

export const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;
export const isWeb = !isTauri;

/**
 * Открыть диалог выбора файла
 * @returns путь к файлу (в Tauri) или имя файла (в вебе)
 */
export async function selectFile(): Promise<string | null> {
  if (isTauri) {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        multiple: false,
        title: 'Выберите файл',
      });
      return typeof selected === 'string' ? selected : null;
    } catch (e) {
      console.error('Tauri dialog error:', e);
      return null;
    }
  } else {
    // Веб-версия: использует HTML input
    return new Promise((resolve) => {
      const input = document.createElement('input');
      input.type = 'file';
      input.onchange = async (e) => {
        const file = (e.target as HTMLInputElement).files?.[0];
        if (file) {
          // В вебе возвращаем имя, а содержимое читается отдельно
          resolve(file.name);
        } else {
          resolve(null);
        }
      };
      input.oncancel = () => resolve(null);
      input.click();
    });
  }
}

/**
 * Получить содержимое файла из drag & drop
 * В Tauri используем listen API вместо HTML5 drag&drop
 */
export async function getFileFromDrop(event: DragEvent): Promise<{ path: string; content?: string } | null> {
  if (!event.dataTransfer) return null;
  
  const file = event.dataTransfer.files?.[0];
  if (!file) return null;

  if (isTauri) {
    // В Tauri у File есть .path
    const path = (file as any)?.path as string | undefined;
    if (path) {
      return { path };
    }
    // Если path нет, пробуем получить через webkitRelativePath или name
    const filePath = (file as any).webkitRelativePath || file.name;
    return filePath ? { path: filePath } : null;
  } else {
    // В вебе читаем содержимое
    const content = await file.text();
    return { path: file.name, content };
  }
}

/**
 * Установить обработчик Tauri file drop events
 */
export async function setupTauriFileDrop(callback: (paths: string[]) => void) {
  console.log('setupTauriFileDrop called, isTauri:', isTauri);
  
  if (isTauri) {
    try {
      console.log('Importing @tauri-apps/api/event...');
      const { listen } = await import('@tauri-apps/api/event');
      
      console.log('Setting up tauri://drag-drop listener...');
      const unlisten = await listen<{ paths: string[]; position: { x: number; y: number } }>('tauri://drag-drop', (event) => {
        console.log('!!! tauri://drag-drop event received:', event.payload);
        // Payload это объект с полем paths
        if (event.payload && event.payload.paths) {
          callback(event.payload.paths);
        }
      });
      
      console.log('Tauri file-drop listener setup complete');
      return unlisten;
    } catch (e) {
      console.error('Failed to setup Tauri file drop:', e);
      return () => {};
    }
  }
  
  console.log('Not in Tauri, skipping file drop setup');
  return () => {};
}

/**
 * Сохранить файл
 */
export async function saveFile(content: string, defaultName: string): Promise<void> {
  if (isTauri) {
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const { writeTextFile } = await import('@tauri-apps/plugin-fs');
      
      const path = await save({
        defaultPath: defaultName,
        title: 'Сохранить файл',
      });
      
      if (path) {
        await writeTextFile(path, content);
      }
    } catch (e) {
      console.error('Tauri save error:', e);
      throw e;
    }
  } else {
    // Веб-версия: скачивание через blob
    const blob = new Blob([content], { type: 'text/plain;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = defaultName;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }
}

/**
 * Копировать в буфер обмена
 */
export async function copyToClipboard(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

/**
 * Вызов Tauri команды (с fallback для веба)
 */
export async function invokeBackend<T = string>(
  command: string,
  args?: Record<string, any>
): Promise<T> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(command, args);
  } else {
    // В вебе используем mock API для демонстрации
    // В продакшене замените на реальный API endpoint
    const { mockInvokeBackend } = await import('./web-mock');
    return mockInvokeBackend<T>(command, args);
  }
}