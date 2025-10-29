# Nooforge UI (universal: Tauri + Browser)

Папка содержит минимальный универсальный фронтенд:
- `/index.html`, `/styles.css`, `/main.js` — чистый HTML/JS/CSS, работает в браузере **и** в Tauri.
- `/src-tauri/src/lib.rs`, `/src-tauri/src/main.rs` — шаблон Tauri-команд под `invoke`.
  Подключите реальные пайплайны (ingest/search) вместо заглушек.

## Запуск в браузере
1. Откройте `index.html` локально или подайте через любой dev-server.
2. Без бэкенда UI использует **моки** — можно сразу проверить UX.

## Запуск в Tauri
1. Перенесите содержимое `index.html`, `styles.css`, `main.js` в папку фронта вашего проекта (`/src-tauri/../dist` или статические assets).
2. Скопируйте `src-tauri/src/lib.rs`, `src-tauri/src/main.rs` (или адаптируйте под вашу структуру).
3. Зарегистрируйте команды и соберите приложение.

## Контракты API (ожидаемые фронтом)
- `ingest_text({ text, lang?, title?, explain? }) -> { chunks: Chunk[], source_id }`
- `ingest_url({ url, lang?, title? }) -> { chunks: Chunk[], source_id }`
- `ingest_file({ name, data_b64, lang?, title? }) -> { chunks: Chunk[], source_id }`
- `search_hybrid({ q, onlyLatest? }) -> { chunks: Chunk[] }`

### Модель Chunk
```json
{
  "id": "string",
  "source": "string",
  "title": "string?",
  "kind": "Text|Code|...",
  "span": [start, end]?,
  "preview": "string?",
  "created_at": "ISO-8601?"
}
```

## Что доработать дальше
- Пагинация ленты, фильтры (kind/source/date), подсветка совпадений.
- Кнопка «Собрать ответ» из выбранных чанков.
- Просмотр полного исходника по `source_id`.
