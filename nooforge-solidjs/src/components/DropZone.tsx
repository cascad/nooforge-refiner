// nooforge-solidjs/src/components/DropZone.tsx
import { createSignal, JSX, onCleanup, onMount } from "solid-js";
import { normalizeDropPaths } from "../lib/path_to_file";

type Props = {
  onPaths: (paths: string[]) => Promise<void> | void;
  class?: string;
};

async function extractViaItems(dt: DataTransfer): Promise<string[]> {
  const items = Array.from(dt.items || []);
  const tasks = items
    .filter(i => i.kind === "string")
    .map(i => new Promise<string>(resolve => {
      try { i.getAsString(s => resolve(s ?? "")); } catch { resolve(""); }
    }));
  if (!tasks.length) return [];
  const arr = await Promise.all(tasks);
  return arr.join("\n").split(/\r?\n/).map(s => s.trim()).filter(Boolean);
}

export default function DropZone(props: Props): JSX.Element {
  const [hover, setHover] = createSignal(false);
  let catcher!: HTMLDivElement; // contenteditable ловушка

  // чтобы VS Code не блокировался политиками — не гасим drop на всём окне
  onMount(() => {
    const allow = (e: DragEvent) => { e.preventDefault(); try { if (e.dataTransfer) e.dataTransfer.dropEffect = "copy"; } catch {} };
    window.addEventListener("dragover", allow, true);
    window.addEventListener("drop", allow, true);
    onCleanup(() => {
      window.removeEventListener("dragover", allow, true);
      window.removeEventListener("drop", allow, true);
    });
  });

  const prevent = (e: DragEvent) => { e.preventDefault(); e.stopPropagation(); };

  const parseAndEmit = async (lines: string[]) => {
    const paths = normalizeDropPaths(lines);
    console.debug("[DropZone] normalized paths:", paths);
    if (paths.length) await props.onPaths(paths);
  };

  const tryDataTransfer = async (dt: DataTransfer | null): Promise<boolean> => {
    if (!dt) return false;

    // 1) стандартные payload’ы
    let text = dt.getData("text/uri-list") || dt.getData("text/plain") || dt.getData("text");
    if (text && text.trim() && text !== "about:blank#blocked") {
      const lines = text.split(/\r?\n/).map(s => s.trim()).filter(Boolean);
      await parseAndEmit(lines);
      return true;
    }

    // 2) VS Code часто кладёт строку через items.getAsString
    const viaItems = await extractViaItems(dt);
    if (viaItems.length) {
      await parseAndEmit(viaItems);
      return true;
    }

    return false;
  };

  const onDragOver = (e: DragEvent) => {
    // тут не стопим дефолт: VS Code должен иметь право «вставить» текст в contenteditable
    e.preventDefault();
    try { if (e.dataTransfer) e.dataTransfer.dropEffect = "copy"; } catch {}
    setHover(true);
  };
  const onDragEnter = onDragOver;
  const onDragLeave = (e: DragEvent) => { prevent(e); setHover(false); };

  const onDrop = async (e: DragEvent) => {
    // НЕ стопим прямо сейчас: сначала пробуем вытащить через DataTransfer,
    // если там мусор (about:blank#blocked) — позволяем упасть в contenteditable.
    e.preventDefault();
    setHover(false);

    const dt = e.dataTransfer ?? null;
    const handled = await tryDataTransfer(dt);
    if (handled) return;

    // Fallback: даём VS Code вставить в contenteditable,
    // потом считываем то, что туда упало.
    // Для этого на мгновение включаем прием дефолтного drop на catcher.
    catcher.innerText = "";
    // Снимаем stopPropagation для внутреннего элемента:
    // просто не делаем ничего — событие уже сработало на контейнере,
    // но WebView2 всё равно вставит текст в contenteditable.
    // Читаем через микротик.
    setTimeout(async () => {
      const raw = catcher.innerText.trim();
      console.debug("[DropZone] fallback contenteditable raw:", JSON.stringify(raw));
      if (raw) {
        const lines = raw.split(/\r?\n/).map(s => s.trim()).filter(Boolean);
        catcher.innerText = "";
        await parseAndEmit(lines);
      } else {
        console.debug("[DropZone] fallback got empty content");
      }
    }, 0);
  };

  return (
    <div
      class={`relative rounded-2xl border-2 ${hover() ? "border-blue-500" : "border-dashed border-white/20"} bg-neutral-950/30 p-12 text-center transition-all group select-none pointer-events-auto ${props.class ?? ""}`}
      onDragOver={onDragOver}
      onDragEnter={onDragEnter}
      onDragLeave={onDragLeave}
      onDrop={onDrop}
      title="Перетащите путь/файл из VS Code или Проводника"
    >
      <div class="text-neutral-400 group-hover:text-neutral-300 transition-colors mb-3">
        Перетащите сюда путь из VS Code или Проводника
      </div>

      {/* Невидимая ловушка для текстового drop-а от VS Code */}
      <div
        ref={catcher}
        contenteditable
        spellcheck={false}
        class="absolute inset-0 opacity-0 pointer-events-none"
        aria-hidden="true"
      />
    </div>
  );
}
