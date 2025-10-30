// nooforge-solidjs/src/lib/dnd.ts
import { invoke } from "@tauri-apps/api/core";

/**
 * Гибрид DnD для WebView2 (Windows):
 * - По умолчанию включён native drop (Explorer/VSCode Explorer) → ловим через tauri://drag-drop
 * - Если видим текстовый drag (VS Code editor: text/plain без Files) → временно выключаем native,
 *   чтобы отработал HTML5 drop в DOM. После drop — включаем обратно.
 *
 * Переключение делаем через backend-команду set_file_drop_enabled(enabled: bool),
 * потому что в v2 нет TS-метода на фронте.
 */
export function setupHybridDnD(): () => void {
  let nativeEnabled = true;
  let toggling = false;

  const setNative = async (enabled: boolean) => {
    if (toggling || nativeEnabled === enabled) return;
    toggling = true;
    try {
      await invoke("set_file_drop_enabled", { enabled });
      nativeEnabled = enabled;
      // console.debug("[DnD] native =", enabled);
    } catch (e) {
      console.error("[DnD] set_file_drop_enabled failed:", e);
    } finally {
      toggling = false;
    }
  };

  const onDragOver = async (e: DragEvent) => {
    // Разрешаем hover всегда
    e.preventDefault();

    const dt = e.dataTransfer;
    if (!dt) return;

    const hasText =
      dt.types?.includes("text/plain") ||
      dt.types?.includes("text/uri-list") ||
      dt.types?.includes("text");
    const hasFiles = dt.types?.includes("Files") || (dt.files && dt.files.length > 0);

    if (hasText && !hasFiles) {
      // VS Code editor → нужен HTML5
      await setNative(false);
      try { dt.dropEffect = "copy"; } catch {}
      return;
    }

    // Иначе — Explorer / VSCode Explorer → держим native
    await setNative(true);
    try { dt.dropEffect = "copy"; } catch {}
  };

  const onDrop = async (_e: DragEvent) => {
    // После drop возвращаем native
    await setNative(true);
  };

  window.addEventListener("dragover", onDragOver, true);
  window.addEventListener("drop", onDrop, true);

  return () => {
    window.removeEventListener("dragover", onDragOver, true);
    window.removeEventListener("drop", onDrop, true);
  };
}
