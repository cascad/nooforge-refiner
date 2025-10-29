# 🤖 RAG System - Полное руководство

Гибридная RAG система на Rust с семантическим поиском и LLM интеграцией.

## ✨ Возможности

- 🔍 **Гибридный поиск**: Семантический (dense vectors) + Keyword matching
- 🤖 **LLM интеграция**: OpenRouter API с Claude Sonnet 4.5
- ⚡ **Streaming**: Получайте ответы в реальном времени
- 🎯 **Умный чанкинг**: Группировка по смыслу, фильтрация мусора
- 🔄 **Автосинхронизация**: Нет дубликатов при переиндексации
- 🚀 **Производительность**: <50ms поиск, ~1000 chunks/sec индексация

## 🚀 Быстрый старт

### 1. Установка зависимостей

```bash
# Запустить Qdrant
docker run -d -p 6334:6334 qdrant/qdrant

# Установить Rust проект
cargo build --release
```

### 2. Настроить API ключ

```bash
# Windows PowerShell
$env:OPENROUTER_API_KEY="sk-or-v1-..."

# Linux/Mac
export OPENROUTER_API_KEY="sk-or-v1-..."
```

Получить ключ: https://openrouter.ai/keys

### 3. Индексировать документы

```bash
cargo run --release --bin ingest -- --input-dir ./docs
```

### 4. Задать вопрос!

```bash
cargo run --release --bin rag -- "Как работает Rust ownership?"
```

## 📖 Подробное использование

### Индексация

```bash
# Индексировать папку
cargo run --bin ingest -- \
  --input-dir ./documents \
  --collection my_docs

# Индексировать с кастомными параметрами
cargo run --bin ingest -- \
  --input-dir ./books \
  --max-tokens 500 \
  --overlap-tokens 100
```

### Поиск (без LLM)

```bash
# Простой семантический поиск
cargo run --bin search -- "machine learning" --limit 5

# Получить контекст для ручной обработки
cargo run --bin search -- "rust programming" --context
```

### RAG с LLM

#### Базовое использование:

```bash
cargo run --bin rag -- "Объясни что такое RAG?"
```

#### С гибридным поиском:

```bash
cargo run --bin rag -- "machine learning algorithms" --hybrid
```

#### Streaming ответ:

```bash
cargo run --bin rag -- "Как работает async в Rust?" --stream
```

#### Показать контекст:

```bash
cargo run --bin rag -- "neural networks" --show-context
```

#### Только контекст (без LLM):

```bash
cargo run --bin rag -- "embedding models" --context-only
```

#### Кастомная модель:

```bash
cargo run --bin rag -- "your question" \
  --model "anthropic/claude-opus-4" \
  --temperature 0.3 \
  --max-tokens 2000
```

#### Больше контекста:

```bash
cargo run --bin rag -- "complex topic" --context-limit 10
```

## 🎯 Примеры workflow

### 1. Базовый RAG pipeline:

```bash
# 1. Индексировать
cargo run --release --bin ingest -- --input-dir ./notes

# 2. Проверить
cargo run --release --bin verify

# 3. Задать вопрос
cargo run --release --bin rag -- "summarize main topics" --stream
```

### 2. Гибридный поиск для точности:

```bash
# Когда нужны exact keywords + семантика
cargo run --release --bin rag -- \
  "find code examples for async" \
  --hybrid \
  --context-limit 7
```

### 3. Интерактивный режим:

```bash
#!/bin/bash
# rag_interactive.sh

while true; do
  read -p "You: " question
  [ "$question" = "exit" ] && break
  
  cargo run --quiet --release --bin rag -- "$question" --stream
  echo ""
done
```

### 4. Batch обработка вопросов:

```bash
# questions.txt
What is Rust?
How does ownership work?
Explain async/await

# process.sh
while IFS= read -r question; do
  echo "Q: $question"
  cargo run --quiet --release --bin rag -- "$question"
  echo "---"
done < questions.txt
```

## 🔧 Параметры

### ingest

| Параметр | Описание | По умолчанию |
|----------|----------|--------------|
| `--input-dir` | Папка с документами | - |
| `--collection` | Имя коллекции | `chunks` |
| `--max-tokens` | Макс токенов на чанк | `350` |
| `--overlap-tokens` | Перекрытие чанков | `60` |
| `--model-dir` | Путь к ONNX модели | `models/multilingual-e5-base` |

### search

| Параметр | Описание | По умолчанию |
|----------|----------|--------------|
| `query` | Поисковый запрос | - |
| `--limit` / `-l` | Кол-во результатов | `5` |
| `--context` | Вывести контекст | - |
| `--format` | `text` или `json` | `text` |

### rag

| Параметр | Описание | По умолчанию |
|----------|----------|--------------|
| `query` | Ваш вопрос | - |
| `--hybrid` | Гибридный поиск | `false` |
| `--stream` | Streaming ответ | `false` |
| `--show-context` | Показать контекст | `false` |
| `--context-only` | Только контекст | `false` |
| `--context-limit` / `-n` | Кол-во чанков | `5` |
| `--model` | LLM модель | `anthropic/claude-sonnet-4.5` |
| `--temperature` | Температура (0.0-2.0) | `0.7` |
| `--max-tokens` | Макс токенов ответа | `4096` |

### verify

```bash
cargo run --bin verify -- --collection chunks
```

## 🌟 Доступные модели (OpenRouter)

```bash
# Claude
--model "anthropic/claude-sonnet-4.5"     # Лучший баланс
--model "anthropic/claude-opus-4"         # Максимальное качество
--model "anthropic/claude-haiku-4"        # Быстрый и дешёвый

# GPT
--model "openai/gpt-4-turbo"
--model "openai/gpt-4o"
--model "openai/gpt-3.5-turbo"

# Open source
--model "meta-llama/llama-3.1-70b-instruct"
--model "mistralai/mistral-large"
--model "google/gemini-pro"
```

Полный список: https://openrouter.ai/models

## 💡 Best Practices

### 1. Качество контекста

```bash
# Для технических вопросов - больше контекста
cargo run --bin rag -- "explain algorithm" --context-limit 10

# Для простых вопросов - меньше контекста
cargo run --bin rag -- "what is X?" --context-limit 3
```

### 2. Температура

```bash
# Точные ответы (факты)
--temperature 0.0

# Баланс
--temperature 0.7

# Креативные ответы
--temperature 1.5
```

### 3. Гибридный поиск

```bash
# Используйте --hybrid когда:
# - Ищете точные термины/имена
# - Нужны code snippets
# - Важны exact keywords

cargo run --bin rag -- "asyncio.create_task example" --hybrid
```

### 4. Streaming

```bash
# Используйте --stream для:
# - Длинных ответов
# - Интерактивного UX
# - Debugging

cargo run --bin rag -- "long explanation" --stream
```

## 🐛 Troubleshooting

### API ключ не работает

```bash
# Проверить
echo $OPENROUTER_API_KEY

# Переустановить
export OPENROUTER_API_KEY="sk-or-v1-..."
```

### Нет результатов поиска

```bash
# Проверить коллекцию
cargo run --bin verify

# Переиндексировать
cargo run --bin ingest -- --input-dir ./docs
```

### Ошибка подключения к Qdrant

```bash
# Проверить что Qdrant запущен
curl http://localhost:6334/healthz

# Перезапустить
docker restart <container_id>
```

### Slow responses

```bash
# Уменьшить context
--context-limit 3

# Использовать быструю модель
--model "anthropic/claude-haiku-4"
```

## 📊 Performance Tips

1. **Используйте --release** для production:
   ```bash
   cargo build --release
   ./target/release/rag "query"
   ```

2. **Кэшируйте частые запросы** на уровне приложения

3. **Настройте chunking** под свои документы:
   - Короткие заметки: `--max-tokens 250`
   - Длинные статьи: `--max-tokens 500`

4. **Batch индексация** для большого объёма:
   ```bash
   find ./docs -name "*.txt" | xargs -P 4 -I {} \
     cargo run --release --bin ingest -- --text "{}"
   ```

## 🎓 Примеры использования

### Для документации проекта

```bash
cargo run --bin ingest -- --input-dir ./docs
cargo run --bin rag -- "How to install?" --stream
```

### Для исследований

```bash
cargo run --bin ingest -- --input-dir ./papers
cargo run --bin rag -- "summarize recent findings on X" \
  --context-limit 10 \
  --temperature 0.3
```

### Для кодовой базы

```bash
cargo run --bin ingest -- --input-dir ./src
cargo run --bin rag -- "explain auth flow" --hybrid --show-context
```

## 🔗 Ссылки

- OpenRouter: https://openrouter.ai
- Qdrant: https://qdrant.tech
- Claude: https://anthropic.com

## 📝 License

MIT