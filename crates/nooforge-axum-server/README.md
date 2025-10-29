# nooforge-axum-server (hybrid)

Сервер Axum, который напрямую использует крейт `hybrid-rag` для ingest/search/RAG (без CLI).

## Эндпоинты
- `POST /api/ingest/text` — JSON: `{ text, lang?, title?, explain? }` → `{ chunks, source_id }`
- `POST /api/ingest/url` — JSON: `{ url, lang?, title? }` → `{ chunks, source_id }`
- `POST /api/ingest/file` — multipart: `file`, `lang?`, `title?` → `{ chunks, source_id }`
- `GET /api/search?q=...&onlyLatest=0|1` → `{ chunks }`
- `GET /health`

## ENV
```
QDRANT_COLLECTION=nooforge
HYBRID_LANG_DEFAULT=ru
BIND_ADDR=127.0.0.1:8090
```

## Сборка
```bash
cargo run --release
```

> В `src/pipeline/hybrid.rs` стоят вызовы `index_text / index_url / index_bytes / search` — если в твоем `hybrid-rag` они называются иначе, замени сигнатуры 1-в-1.

-------

Установи CUDA Toolkit 12.x (под свою версию ORT GPU-сборки).

Установи/распакуй cuDNN для той же версии CUDA.

Убедись, что в PATH или рядом с .exe есть следующие DLL (имена могут чуть отличаться по версии):

cudart64_12*.dll

cublas64_12*.dll, cublasLt64_12*.dll

cudnn64*.dll

Добавь в PATH папку с ORT GPU DLL:


---

$env:ORT_USE="cuda"
$env:ORT_DYLIB_PATH="F:\\libs\\onnxruntime-win-x64-gpu-1.23.2\\lib"
$env:PATH = "$env:ORT_DYLIB_PATH;$env:PATH"

# crates/hybrid-rag/Cargo.toml
[dependencies]
ort = { version = "1.16", features = ["load-dynamic", "cuda"]

```rust
use ort::execution_providers::{CudaExecutionProviderOptions, ExecutionProvider};

let mut b = env
    .new_session_builder()?
    .with_optimization_level(ort::GraphOptimizationLevel::All)?;
b = b.with_execution_providers([ExecutionProvider::Cuda(
    CudaExecutionProviderOptions::default(),
)])?;
```

(Опционально) Жёстко проверяй, что CUDA реально поднялся:
```rust
let eps = ort::session::registered_execution_providers()?;
assert!(eps.iter().any(|e| e.contains("CUDA")), "CUDA EP not registered");
```


Invoke-RestMethod -Uri "http://127.0.0.1:8090/health" -Method GET

Ingest текст (вставка документа в индекс)
$body = @{
    text  = "Это тестовый документ для проверки индексатора."
    title = "demo_doc"
} | ConvertTo-Json

Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/ingest/text" `
  -Method POST `
  -ContentType "application/json" `
  -Body $body\


Search (поиск по тексту / RAG-векторке)
Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/search?q=проверка&limit=5" -Method GET

RAG-запрос (LLM с retrieval)
$body = @{
    q = "О чём был тестовый документ demo_doc?"
    limit = 5
} | ConvertTo-Json

Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/rag" `
  -Method POST `
  -ContentType "application/json" `
  -Body $body

[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

curl -X DELETE "http://127.0.0.1:6333/collections/chunks"
Invoke-RestMethod -Uri "http://127.0.0.1:6333/collections/chunks" -Method DELETE


RAG-запрос (LLM с retrieval)

$body = @{
    q = "Про русский текст?"
    limit = 5
} | ConvertTo-Json

Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/rag" `
  -Method POST `
  -ContentType "application/json" `
  -Body $body

curl -s -X POST "http://$Q/collections/$COL/points/scroll" `
  -H "Content-Type: application/json" `
  --data-binary "$scroll"


------
рабочий вариант

"привет кириллица" | Out-File raw.txt -Encoding default

$raw = [System.IO.File]::ReadAllBytes("$PWD\raw.txt")

Invoke-RestMethod -Uri "http://127.0.0.1:8090/api/ingest/text_raw?title=PS&lang=ru" `
   -Method POST `
   -ContentType "application/octet-stream" `
   -Body $raw

----

# Включим нормальный UTF-8 вывод (чтобы логи в консоли читались)
chcp 65001 > $null
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

# 1) Русский текст в CP1251 (проверяем декодер)
"Компакция в LSM-tree бывает leveled и tiered. Leveled снижает read amplification, tiered экономит запись." |
  Out-File -FilePath lsm_cp1251.txt -Encoding default

# 2) Отвлечённый документ про Bevy (UTF-8)
"Bevy — игровой движок на Rust. Он не имеет отношения к LSM-tree." |
  Out-File -FilePath bevy_utf8.txt -Encoding utf8