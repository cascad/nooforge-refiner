# RAG System with LlamaIndex + Qdrant

Complete production-ready RAG system with semantic chunking.

## Quick Start

1. **Setup**
   ```bash
   chmod +x setup.sh
   ./setup.sh
   ```

2. **Add documents**
   ```bash
   cp your_documents.txt data/
   ```

3. **Ingest**
   ```bash
   python ingest.py --data data/
   ```

4. **Query**
   ```bash
   # Interactive mode
   python query.py --interactive
   
   # Single query
   python query.py "Что такое нейросеть личность?"
   
   # Search only (no LLM)
   python query.py --search-only "semantic chunking"
   ```

## Architecture

```
Documents → SemanticSplitter → Embeddings → Qdrant
                                                ↓
Query → Embedding → Vector Search → Top-K → LLM → Answer
```

## Configuration

Edit `.env` file:
- `EMBED_MODEL`: Embedding model (multilingual for Russian)
- `SEMANTIC_THRESHOLD`: 90-99 (higher = larger chunks)
- `TOP_K`: Number of results to retrieve

## Commands

```bash
# Reset and re-ingest
python ingest.py --reset

# Search without LLM
python query.py --search-only "your query" --top-k 10

# Stop Qdrant
docker-compose down

# View Qdrant UI
open http://localhost:6333/dashboard
```

## For Russian Documents

In `.env`:
```
EMBED_MODEL=intfloat/multilingual-e5-small
```

## Troubleshooting

**Qdrant not starting:**
```bash
docker-compose logs qdrant
```


python -m pip install huggingface_hub[hf_xet]

python -m pip install -U llama-index-llms-openrouter

**Slow embedding:**
- Use smaller model: `all-MiniLM-L6-v2`
- Or add GPU support to docker-compose.yml


.\llamaindex_full\setup_minimal.ps1


copy .env.example .env
notepad .env
# Вставь свой OPENROUTER_API_KEY=sk-or-v1-xxxxx
# И для русского: EMBED_MODEL=intfloat/multilingual-e5-small

# 2. Скопируй документы
copy D:\rust_repo\nooforge-refiner\llm_segmentation\data\*.txt data\

# 3. Загрузи в векторную БД
python ingest.py

# 4. Попробуй запросы
python query.py --interactive


----

# Проверь статус Qdrant
docker-compose logs qdrant

# Подожди 10-20 секунд и проверь здоровье
curl http://localhost:6333/healthz

# Или через PowerShell
Invoke-WebRequest -Uri http://localhost:6333/healthz

# Когда ответит - попробуй снова
python ingest.py

---------

# Проверь что Qdrant реально работает
Invoke-WebRequest http://localhost:6333/collections

# Или через curl если есть
curl http://localhost:6333/collections

# Посмотри что в контейнере
docker-compose ps

# Перезапусти Qdrant
docker-compose down
docker-compose up -d

# Подожди 10 секунд
Start-Sleep -Seconds 10

# Проверь что отвечает
curl http://localhost:6333/collections

# Если отвечает {"result":{"collections":[]}} - норм
# Тогда снова:
python ingest.py


$env:NO_PROXY="127.0.0.1,localhost"
$env:HTTP_PROXY=""
$env:HTTPS_PROXY=""
python ingest.py