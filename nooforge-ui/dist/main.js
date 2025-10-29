// /main.js
// Универсальный фронт: работает и в Tauri, и в обычном браузере.
// Endpoints (ожидаются на бэке):
//   POST /api/ingest/text_raw   (body: raw bytes, query: lang,title,source_id?,explain?)
//   POST /api/ingest/file       (multipart: file + title + lang)
//   POST /api/rag               (json: { q, limit? })
//   GET  /api/search            (query: q, limit?)

// ===== Helpers: Tauri invoke fallback =====
const isTauri = !!window.__TAURI__;
async function apiFetch(url, opts) {
  if (!isTauri) {
    return fetch(url, opts);
  }
  // Если хочешь, можно тут делать invoke в Tauri-команды.
  // Сейчас для простоты Tauri тоже пользуется fetch (разреши домен в tauri.conf.json).
  return fetch(url, opts);
}

function qs(id) { return document.getElementById(id); }
function setStatus(text) { qs('status').textContent = text || ''; }

// ===== Top-level tabs =====
document.querySelectorAll('.top-tab').forEach(btn => {
  btn.addEventListener('click', () => {
    const screen = btn.dataset.screen;
    document.querySelectorAll('.top-tab').forEach(b => b.classList.remove('active'));
    btn.classList.add('active');
    document.querySelectorAll('.screen').forEach(s => s.classList.add('hidden'));
    document.getElementById(screen).classList.remove('hidden');
  });
});

// ===== Ingest subtabs =====
document.querySelectorAll('.tab').forEach(btn => {
  btn.addEventListener('click', () => {
    const tab = btn.dataset.tab;
    document.querySelectorAll('.tab').forEach(b => b.classList.remove('active'));
    btn.classList.add('active');
    document.querySelectorAll('.tab-content').forEach(s => s.classList.add('hidden'));
    document.getElementById(`tab-${tab}`).classList.remove('hidden');
  });
});

// ===== Populate chunks table =====
function renderChunks(chunks = []) {
  const tbody = qs('chunks-tbody');
  tbody.innerHTML = '';
  chunks.forEach(ch => {
    const tr = document.createElement('tr');
    tr.innerHTML = `
      <td title="${ch.source || ''}">${ch.source || ''}</td>
      <td>${ch.title || ''}</td>
      <td>${ch.kind || ''}</td>
      <td>${ch.span ? `[${ch.span[0]}, ${ch.span[1]}]` : ''}</td>
      <td>${ch.created_at || ''}</td>
    `;
    tr.addEventListener('click', () => {
      document.querySelectorAll('#chunks-tbody tr').forEach(r => r.classList.remove('selected'));
      tr.classList.add('selected');
      qs('chunk-preview').textContent = ch.preview || '';
      qs('preview-meta').textContent = `${ch.source || ''} ${ch.title ? '· ' + ch.title : ''}`;
    });
    tbody.appendChild(tr);
  });
}

// ===== INGEST: text =====
qs('ingest-text-btn').addEventListener('click', async () => {
  const text = qs('ingest-text').value || '';
  const title = qs('ingest-title').value || '';
  const lang = qs('ingest-lang').value || 'ru';
  const explain = qs('ingest-explain').checked ? 'true' : 'false';

  if (!text.trim()) { setStatus('Пустой текст'); return; }

  setStatus('Загрузка текста…');
  try {
    const url = `http://127.0.0.1:8090/api/ingest/text_raw?lang=${encodeURIComponent(lang)}&title=${encodeURIComponent(title)}&explain=${explain}`;
    const resp = await apiFetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'text/plain; charset=utf-8' },
      body: new TextEncoder().encode(text) // байты UTF-8
    });
    const data = await resp.json();
    renderChunks(data.chunks || []);
    setStatus(`Готово: ${data.chunks?.length ?? 0} чанков`);
  } catch (e) {
    console.error(e);
    setStatus('Ошибка загрузки текста');
  }
});

// ===== INGEST: URL =====
qs('ingest-url-btn').addEventListener('click', async () => {
  const urlVal = qs('ingest-url').value || '';
  const title = qs('ingest-url-title').value || '';
  const lang = qs('ingest-url-lang').value || 'ru';
  if (!urlVal) { setStatus('Пустой URL'); return; }
  setStatus('Загрузка URL…');
  try {
    const resp = await apiFetch('http://127.0.0.1:8090/api/ingest/url', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json; charset=utf-8' },
      body: JSON.stringify({ url: urlVal, title, lang })
    });
    const data = await resp.json();
    renderChunks(data.chunks || []);
    setStatus(`Готово: ${data.chunks?.length ?? 0} чанков`);
  } catch (e) {
    console.error(e);
    setStatus('Ошибка загрузки URL');
  }
});

// ===== INGEST: file (tab-file, multipart) =====
qs('ingest-file-btn').addEventListener('click', async () => {
  const fileInput = qs('ingest-file-raw');
  if (!fileInput.files?.length) { setStatus('Файл не выбран'); return; }
  const f = fileInput.files[0];
  const title = qs('ingest-file-title').value || '';
  const lang = qs('ingest-file-lang').value || 'ru';

  const form = new FormData();
  form.append('file', f, f.name);
  form.append('title', title);
  form.append('lang', lang);

  setStatus('Загрузка файла…');
  try {
    const resp = await apiFetch('http://127.0.0.1:8090/api/ingest/file', {
      method: 'POST',
      body: form
    });
    const data = await resp.json();
    renderChunks(data.chunks || []);
    setStatus(`Готово: ${data.chunks?.length ?? 0} чанков`);
  } catch (e) {
    console.error(e);
    setStatus('Ошибка загрузки файла');
  }
});

// ===== INGEST: picker (кнопка возле textarea) =====
qs('ingest-file-picker-btn').addEventListener('click', () => {
  qs('ingest-file').click();
});
qs('ingest-file').addEventListener('change', async (ev) => {
  const f = ev.target.files?.[0];
  if (!f) return;
  // кидаем его как multipart в /api/ingest/file
  const form = new FormData();
  form.append('file', f, f.name);
  form.append('title', qs('ingest-title').value || '');
  form.append('lang', qs('ingest-lang').value || 'ru');
  setStatus('Загрузка файла…');
  try {
    const resp = await apiFetch('http://127.0.0.1:8090/api/ingest/file', { method: 'POST', body: form });
    const data = await resp.json();
    renderChunks(data.chunks || []);
    setStatus(`Готово: ${data.chunks?.length ?? 0} чанков`);
  } catch (e) {
    console.error(e);
    setStatus('Ошибка загрузки файла');
  }
});

// ===== INGEST: drag&drop в textarea (как просил — сюда можно кидать файл) =====
const dropTargets = [qs('ingest-text'), qs('file-drop-zone')];
dropTargets.forEach(el => {
  if (!el) return;
  el.addEventListener('dragover', (e) => { e.preventDefault(); el.classList.add('drag'); });
  el.addEventListener('dragleave', () => el.classList.remove('drag'));
  el.addEventListener('drop', async (e) => {
    e.preventDefault(); el.classList.remove('drag');
    const file = e.dataTransfer.files?.[0];
    if (!file) return;
    const title = qs('ingest-title')?.value || qs('ingest-file-title')?.value || '';
    const lang = qs('ingest-lang')?.value || qs('ingest-file-lang')?.value || 'ru';

    const form = new FormData();
    form.append('file', file, file.name);
    form.append('title', title);
    form.append('lang', lang);
    setStatus('Загрузка файла (drop)…');
    try {
      const resp = await apiFetch('http://127.0.0.1:8090/api/ingest/file', { method: 'POST', body: form });
      const data = await resp.json();
      renderChunks(data.chunks || []);
      setStatus(`Готово: ${data.chunks?.length ?? 0} чанков`);
    } catch (e) {
      console.error(e);
      setStatus('Ошибка загрузки файла (drop)');
    }
  });
});

// ===== RAG =====
let ragController = null;
qs('rag-run-btn').addEventListener('click', async () => {
  const q = qs('rag-input').value || '';
  if (!q.trim()) { return; }
  qs('rag-output').textContent = '';
  qs('rag-context').textContent = '';

  ragController = new AbortController();
  try {
    const resp = await apiFetch('http://127.0.0.1:8090/api/rag', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json; charset=utf-8' },
      body: JSON.stringify({ q, limit: 6 }),
      signal: ragController.signal,
    });
    const data = await resp.json();
    qs('rag-output').textContent = data.answer || '';
    qs('rag-context').textContent = data.context || '';
  } catch (e) {
    if (e.name === 'AbortError') {
      qs('rag-output').textContent = '[Остановлено]';
    } else {
      console.error(e);
      qs('rag-output').textContent = '[Ошибка запроса]';
    }
  } finally {
    ragController = null;
  }
});
qs('rag-cancel-btn').addEventListener('click', () => {
  if (ragController) ragController.abort();
});

// ===== SEARCH =====
let searchController = null;
qs('search-run-btn').addEventListener('click', async () => {
  const q = qs('search-query').value || '';
  if (!q.trim()) return;

  qs('search-results').innerHTML = '';
  qs('search-preview').textContent = '';

  searchController = new AbortController();
  try {
    const url = `http://127.0.0.1:8090/api/search?q=${encodeURIComponent(q)}&limit=10`;
    const resp = await apiFetch(url, { method: 'GET', signal: searchController.signal });
    const data = await resp.json();

    const list = qs('search-results');
    (data.results || data.chunks || []).forEach(item => {
      const li = document.createElement('li');
      li.textContent = `${item.source || ''} · ${item.title || ''}`;
      li.addEventListener('click', () => {
        qs('search-preview').textContent = item.preview || '';
      });
      list.appendChild(li);
    });
  } catch (e) {
    if (e.name === 'AbortError') {
      qs('search-preview').textContent = '[Остановлено]';
    } else {
      console.error(e);
      qs('search-preview').textContent = '[Ошибка запроса]';
    }
  } finally {
    searchController = null;
  }
});
qs('search-cancel-btn').addEventListener('click', () => {
  if (searchController) searchController.abort();
});
