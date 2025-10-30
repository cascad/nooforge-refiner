// nooforge-solidjs/src/lib/platform.ts
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";

/** Универсальная обёртка над invoke с логами (видно в DevTools). */
export async function invokeBackend<T = unknown>(
  cmd: string,
  payload?: Record<string, unknown>
): Promise<T> {
  console.debug("[invoke] →", cmd, payload ?? {});
  try {
    const res = await invoke<T>(cmd, payload as any);
    console.debug("[invoke] ←", cmd, res);
    return res;
  } catch (err) {
    console.error("[invoke] ✖", cmd, err);
    throw err;
  }
}

/** Диалог выбора одного файла (Tauri v2: plugin-dialog). */
export async function selectFile(): Promise<string | null> {
  try {
    const result = await open({ multiple: false, directory: false });
    if (typeof result === "string") return result;
    return null;
  } catch (e) {
    console.error("[selectFile] failed:", e);
    return null;
  }
}

/** Подписка на нативный drop от ОС: tauri://drag-drop (Проводник/Файндер). */
export async function setupTauriFileDrop(
  handler: (paths: string[]) => void | Promise<void>
): Promise<() => void> {
  const unlisten = await listen<{ paths?: string[] }>("tauri://drag-drop", async (ev) => {
    const p = Array.isArray(ev.payload?.paths) ? ev.payload!.paths! : [];
    console.debug("[tauri://drag-drop]", p);
    if (p.length) await handler(p);
  });
  return () => {
    try { unlisten(); } catch { /* noop */ }
  };
}

/** Копирование текста в буфер обмена (Tauri v2 + фолбэк на браузер). */
export async function copyToClipboard(text: string): Promise<boolean> {
  try {
    await writeText(text);
    return true;
  } catch (e1) {
    try {
      await navigator.clipboard.writeText(text);
      return true;
    } catch (e2) {
      console.error("[copyToClipboard] failed:", e1, e2);
      return false;
    }
  }
}
