// nooforge-solidjs/src/lib/path_to_file.ts

/** Нормализует все популярные форматы путей (VS Code, file://, /C:/…, %3A) в нормальный Windows/Posix путь. */
export function normalizeDropPath(input: string): string {
  let s = (input || "").trim();
  if (!s) return s;

  // vscode-file:// → file://
  if (s.startsWith("vscode-file://")) {
    const idx = s.indexOf("/", "vscode-file://".length);
    s = idx >= 0 ? "file://" + s.slice(idx) : s.replace(/^vscode-file:\/\//, "file://");
  }

  // file://… → абсолютный путь
  if (s.startsWith("file://")) {
    s = s.replace(/^file:\/\//, "");  // убрать схему
    s = s.replace(/^\/+/, "/");       // ///C:/x → /C:/x
    try { s = decodeURI(s); } catch {}
    // /C:/x → C:/x
    const m = /^\/([A-Za-z]:\/.*)$/.exec(s);
    if (m) s = m[1];
  }

  // /c%3A/x → c:/x
  if (/^\/[A-Za-z]%3A\//i.test(s)) {
    try { s = decodeURIComponent(s); } catch {}
    s = s.replace(/^\/([A-Za-z]:\/)/, "$1");
  }

  // убрать случайные кавычки/пробелы/CRLF
  s = s.replace(/^\s*"+|"+\s*$/g, "").replace(/\r/g, "");

  return s;
}

/** Нормализует массив с отбрасыванием пустых значений. */
export function normalizeDropPaths(lines: string[]): string[] {
  return lines.map(normalizeDropPath).filter(Boolean);
}
