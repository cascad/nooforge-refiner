# ✅ Установка RAG системы - Чеклист

## 1️⃣ Скопировать файлы

```bash
# Основные модули
cp llm.rs crates/rag1/src/
cp bin_rag.rs crates/rag1/src/bin/rag.rs
cp query.rs crates/rag1/src/
cp lib.rs crates/rag1/src/

# Обновить chunking (с исправлением UTF-8)
cp chunking_v2.rs crates/rag1/src/chunking.rs

# Cargo.toml с новыми зависимостями
cp Cargo_updated.toml crates/rag1/Cargo.toml

# Документация
cp RAG_GUIDE.md ./
cp RAG_SUMMARY.md ./
```

## 2️⃣ Обновить зависимости

Cargo.toml уже обновлен с:
- ✅ `reqwest` с feature `"stream"`
- ✅ `futures-util = "0.3"`

```bash
cd crates/rag1
cargo update
```

## 3️⃣ Собрать проект

```bash
cargo build --release
```

Должно собраться без ошибок!

## 4️⃣ Настроить окружение

### Windows PowerShell:
```powershell
$env:OPENROUTER_API_KEY="sk-or-v1-..."
```

### Linux/Mac:
```bash
export OPENROUTER_API_KEY="sk-or-v1-..."

# Добавить в ~/.bashrc или ~/.zshrc для постоянного использования
echo 'export OPENROUTER_API_KEY="sk-or-v1-..."' >> ~/.bashrc
```

### Получить ключ:
https://openrouter.ai/keys

## 5️⃣ Запустить Qdrant

```bash
docker run -d -p 6334:6334 qdrant/qdrant
```

Проверить:
```bash
curl http://localhost:6334/healthz
```

## 6️⃣ Индексировать данные

```bash
cargo run --release --bin ingest -- --input-dir ./notes
```

## 7️⃣ Проверить работу

```bash
# Проверить индекс
cargo run --release --bin verify

# Простой поиск
cargo run --release --bin search -- "rust" --limit 5

# RAG с LLM
cargo run --release --bin rag -- "Как работает Rust?" --stream
```

## 🎯 Пример полного workflow:

```bash
# 1. Скопировать все файлы (см. выше)

# 2. Собрать
cd crates/rag1
cargo build --release

# 3. Настроить API ключ
export OPENROUTER_API_KEY="sk-or-v1-..."

# 4. Запустить Qdrant
docker run -d -p 6334:6334 qdrant/qdrant

# 5. Индексировать
cargo run --release --bin ingest -- --input-dir ./notes

# 6. Задать вопрос!
cargo run --release --bin rag -- "Объясни концепцию ownership в Rust" --stream
```

## 🐛 Возможные проблемы:

### Ошибка компиляции llm.rs

**Проблема:** `bytes_stream` not found

**Решение:** Убедитесь что в Cargo.toml:
```toml
reqwest = { version = "0.11", features = ["json", "stream"] }
futures-util = "0.3"
```

Затем:
```bash
cargo clean
cargo build --release
```

### API ключ не работает

**Проверить:**
```bash
# Windows
echo $env:OPENROUTER_API_KEY

# Linux/Mac  
echo $OPENROUTER_API_KEY
```

**Установить заново:**
```bash
export OPENROUTER_API_KEY="sk-or-v1-..."
```

### Qdrant не подключается

**Проверить что запущен:**
```bash
docker ps | grep qdrant
curl http://localhost:6334/healthz
```

**Перезапустить:**
```bash
docker restart <container_id>
```

### Нет результатов поиска

**Переиндексировать:**
```bash
cargo run --release --bin ingest -- --input-dir ./notes
cargo run --release --bin verify
```

## 📊 Что должно работать:

✅ **Компиляция без ошибок**
```bash
cargo build --release
# Компилируется успешно
```

✅ **Индексация**
```bash
cargo run --bin ingest -- --input-dir ./notes
# 🚀 Initializing indexer...
# ✅ Indexed X documents, Y chunks
```

✅ **Поиск**
```bash
cargo run --bin search -- "rust"
# ✅ Found 5 results:
# 1. Score: 0.85 ...
```

✅ **RAG с LLM**
```bash
cargo run --bin rag -- "test question" --stream
# 🤖 RAG System
# 📝 Question: test question
# 🔍 Searching knowledge base...
# ✅ Found 5 relevant chunks
# 💭 Generating answer...
# [Ответ появляется по мере генерации]
# ✨ Done!
```

## 🚀 Следующие шаги:

1. **Прочитать документацию:**
   - RAG_GUIDE.md - подробное руководство
   - RAG_SUMMARY.md - сводка и примеры

2. **Попробовать разные режимы:**
   ```bash
   # Гибридный поиск
   cargo run --bin rag -- "query" --hybrid
   
   # Показать контекст
   cargo run --bin rag -- "query" --show-context
   
   # Только контекст без LLM
   cargo run --bin rag -- "query" --context-only
   
   # Кастомная модель
   cargo run --bin rag -- "query" --model "anthropic/claude-opus-4"
   ```

3. **Настроить под свои данные:**
   - Изменить chunking параметры
   - Экспериментировать с temperature
   - Попробовать разные модели

4. **Создать Web API** (следующий шаг):
   - REST API с Axum
   - WebSocket для streaming
   - Frontend с React/Vue

## 📚 Ресурсы:

- OpenRouter: https://openrouter.ai
- Qdrant: https://qdrant.tech
- Rust async book: https://rust-lang.github.io/async-book/

## ✨ Готово!

Теперь у вас полноценная RAG система с:
- ✅ Гибридным поиском
- ✅ LLM интеграцией (Claude Sonnet 4.5)
- ✅ Streaming ответами
- ✅ Умным чанкингом
- ✅ Production-ready кодом

Удачи! 🚀