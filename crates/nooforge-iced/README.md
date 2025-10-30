# Nooforge Iced UI

Нативный GUI на Rust + iced для Nooforge.

## Возможности

- ✅ Ingest text
- ✅ Ingest file (кнопка выбора)
- ✅ RAG запросы
- ✅ Search
- ✅ Работает на Linux, macOS, Windows
- 🚧 Drag & Drop (нужно добавить)

## Запуск

```bash
cd nooforge-iced
cargo run
```

## Сборка релиза

```bash
cargo build --release
```

Бинарник будет в `target/release/nooforge-iced` (или `.exe` на Windows)

## Drag & Drop

**TODO:** Iced 0.12 поддерживает drag&drop, но требует subscription.
Нужно добавить:

```rust
fn subscription(&self) -> Subscription<Message> {
    iced::window::events().map(|event| {
        if let iced::window::Event::FileDropped(path) = event {
            Message::IngestFileDropped(path.display().to_string())
        } else {
            Message::Noop
        }
    })
}
```

## Drag текста из VSCode

Когда дрэгаешь файл из VSCode, он отправляет путь как строку.
Обработчик `IngestFileDropped` принимает `String` и конвертит в `PathBuf`.

## API

Backend должен быть запущен на `http://127.0.0.1:8090`:
- POST `/api/ingest/text` - `{ "text": "..." }`
- POST `/api/ingest/file` - multipart form с файлом
- POST `/api/rag` - `{ "q": "query", "limit": 5 }`
- GET `/api/search?q=query&limit=10`
