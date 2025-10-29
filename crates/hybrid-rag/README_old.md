# Загрузка одного файла
cargo run -- ingest data/doc1.txt

# Загрузка всей папки
cargo run -- ingest data/

# Загрузка с указанием коллекции
cargo run -- ingest data/ --collection my_docs

# Поиск (когда реализуем)
cargo run -- query "настройка VPN"

https://huggingface.co/intfloat/multilingual-e5-base/blob/a114a4100c6714cf21651971eefe9191a4415dbb/onnx/model.onnx?utm_source=chatgpt.com

wget https://huggingface.co/intfloat/multilingual-e5-base/resolve/main/tokenizer.json
wget https://huggingface.co/intfloat/multilingual-e5-base/resolve/main/tokenizer_config.json
wget https://huggingface.co/intfloat/multilingual-e5-base/resolve/main/special_tokens_map.json
wget https://huggingface.co/intfloat/multilingual-e5-base/resolve/main/sentencepiece.bpe.model
wget https://huggingface.co/intfloat/multilingual-e5-base/resolve/main/onnx/model.onnx

Содержит hidden_size, max_position_embeddings, vocab_size, architectures, model_type, layer_norm_eps и т.д. — нужен, если ты хочешь в Rust собрать структуру модели (например, для tch или onnx shape checking).
https://huggingface.co/intfloat/multilingual-e5-base/resolve/main/config.json

Необязателен, но полезен: там хранится информация о том, как pooling делается (mean/cls), какие normalization шаги применяются. Если ты хочешь 100% совпадение эмбеддингов с Python, нужен.
https://huggingface.co/intfloat/multilingual-e5-base/resolve/main/modules.json


onnxruntime
https://github.com/microsoft/onnxruntime/releases/latest

F:\libs\onnxruntime-win-x64-gpu-1.23.2\lib

$env:ORT_USE="directml" # если без CUDA

$env:ORT_USE="cuda"
$env:ORT_DYLIB_PATH="F:\libs\onnxruntime-win-x64-gpu-1.23.2\lib"
$env:PATH = "$env:ORT_DYLIB_PATH;$env:PATH"

отстрелить rust-analyzer
taskkill /F /IM rust-analyzer.exe 2>$null


cargo run --release -- ^
  --model-dir "models/multilingual-e5-base" ^
  --tokenizer-path "models/multilingual-e5-base/tokenizer.json" ^
  --text "Привет, мир! Проверка эмбеддингов."

------

# если нужна GPU-версия
$env:ORT_USE="cuda"            # или "directml" на Windows без CUDA
$env:ORT_DYLIB_PATH="C:\tools\onnxruntime-win-x64-gpu-1.17.0\lib"
$env:PATH="$env:ORT_DYLIB_PATH;$env:PATH"

cargo clean
cargo run --release -- `
  --model-dir models `
  --tokenizer-path models/tokenizer.json `
  --text "Привет, мир! Проверка эмбеддингов."

создать коллекцию в Qdrant

curl -X PUT "http://127.0.0.1:6333/collections/chunks" `
  -H "Content-Type: application/json" `
  -d '{
    "vectors": { "size": 768, "distance": "Cosine" },
    "optimizers_config": { "default_segment_number": 2 }
  }'

Invoke-RestMethod -Uri "http://127.0.0.1:6333/collections/chunks" `
  -Method PUT `
  -ContentType "application/json" `
  -Body '{
    "vectors": {
      "size": 768,
      "distance": "Cosine"
    }
  }'

Invoke-RestMethod -Uri "http://127.0.0.1:6333/collections" -Method GET



cargo run --release --bin ingest_qdrant -- `
  --text "Компакция в LSM-tree бывает leveled и tiered." `
  --source-id "samples/lsm-notes.md" `
  --collection "chunks" `
  --model-dir "models" `
  --tokenizer-path "models/tokenizer.json" `
  --qdrant-host "127.0.0.1" `
  --qdrant-grpc-port 6334


cargo run --release --bin search_qdrant -- `
  --query "как работает компакция в LSM-tree?" `
  --collection "chunks" `
  --model-dir "models" `
  --tokenizer-path "models/tokenizer.json" `
  --qdrant-host "127.0.0.1" `
  --qdrant-grpc-port 6334 `
  --top-k 5 `
  --score-threshold 0.2 `
  --with-payload true

cargo run --release --bin search_qdrant -- `
  --query "как работает компакция в LSM-tree?" `
  --collection chunks `
  --model-dir models `
  --tokenizer-path models/tokenizer.json `
  --qdrant-host 127.0.0.1 `
  --qdrant-grpc-port 6334 `
  --top-k 5 `
  --with-payload

batch-ingest

cargo run --release --bin ingest_qdrant -- `
  --input-dir "./notes" `
  --collection "chunks" `
  --model-dir "models" `
  --tokenizer-path "models/tokenizer.json" `
  --qdrant-host "127.0.0.1" `
  --qdrant-grpc-port 6334

--------


Тюнинг коллекции (HNSW + оптимайзер)

Invoke-RestMethod -Uri "http://127.0.0.1:6333/collections/chunks/params" `
  -Method PATCH -ContentType "application/json" `
  -Body '{
    "hnsw_config": { "m": 16, "ef_construct": 200 },
    "optimizer_config": { "memmap_threshold": 200000, "default_segment_number": 2 },
    "quantization_config": null
  }'

----


$env:QDRANT_HOST="127.0.0.1"
$env:QDRANT_GRPC_PORT="6334"

cargo run --release --bin ingest_qdrant_grpc_chunked -- `
  --input-dir .\samples `
  --source-id "file://samples" `
  --collection chunks `
  --model-dir .\models\multilingual-e5-base `
  --tokenizer-path .\models\multilingual-e5-base\tokenizer.json `
  --wait

QDRANT_HOST=127.0.0.1 QDRANT_GRPC_PORT=6334 \
cargo run --release --bin ingest_qdrant_grpc_chunked \
  --input-dir ./samples \
  --source-id "file://samples" \
  --collection chunks \
  --model-dir ./models/multilingual-e5-base \
  --tokenizer-path ./models/multilingual-e5-base/tokenizer.json \
  --wait

ingest_qdrant_chunked_v3.rs

cargo run --release --bin ingest_qdrant_chunked_v3 -- `
  --input-dir ./notes `
  --source-id "file://samples" `
  --collection chunks `
  --model-dir ./models `
  --tokenizer-path ./models/tokenizer.json

---------

docker run -p 6333:6333 -p 6334:6334 qdrant/qdrant

# Откройте в браузере
http://localhost:6333/dashboard


# Или с полными путями к модели
cargo run --bin ingest -- `
  --input-dir ./notes `
  --collection chunks `
  --model-dir ./models/multilingual-e5-base `
  --tokenizer-path ./models/multilingual-e5-base/tokenizer.json `
  --source-id "file://notes"


cargo run --bin verify -- --collection chunks
```

Вывод будет примерно таким:
```
🔍 Analyzing collection: chunks

📊 Total points: 180
📚 Total documents: 2
📄 Total chunks: 180

🔎 Checking for duplicate chunks...
✅ No duplicate chunks found!

📊 Top 5 documents by chunk count:
  1. doc::230a81772d646590a - 130 chunks
  2. doc::9bc50254b1f95d058 - 50 chunks

📁 Documents by source:
  file://samples/ai_article.txt - 130 chunks
  file://samples/rust_guide.txt - 50 chunks

✨ Verification complete!


# Запустите ingest несколько раз на те же файлы
cargo run --bin ingest -- --input-dir ./notes
cargo run --bin ingest -- --input-dir ./notes
cargo run --bin ingest -- --input-dir ./notes

# Проверьте - количество должно остаться тем же!
cargo run --bin verify


------

# Для коротких документов (статьи, заметки)
cargo run --bin ingest -- \
  --input-dir ./docs \
  --max-tokens 250 \
  --overlap-tokens 40

# Для длинных документов (книги, отчёты)
cargo run --bin ingest -- \
  --input-dir ./books \
  --max-tokens 500 \
  --overlap-tokens 100

--------

Проверка качества чанкинга

# Посмотрите на реальные чанки
cargo run --bin verify -- --collection chunks

# Поищите что-то конкретное
cargo run --bin search -- "rust programming" --limit 10

# Получите контекст для RAG
cargo run --bin search -- "machine learning" --context --limit 5


cargo run --release --bin rag -- "test question" --stream


-----

чистим коллекцию clean collection

curl -X DELETE "http://127.0.0.1:6333/collections/chunks"
Invoke-RestMethod -Uri "http://127.0.0.1:6333/collections/chunks" -Method DELETE


текст

cargo run --bin ingest -- `
  --text "Компакция в LSM-tree бывает leveled и tiered." `
  --collection chunks `
  --model-dir ./models/multilingual-e5-base `
  --tokenizer-path ./models/multilingual-e5-base/tokenizer.json `
  --source-id "file://notes"

папку

cargo run --bin ingest -- `
  --input-dir ./docs `
  --collection chunks `
  --model-dir ./models/multilingual-e5-base `
  --tokenizer-path ./models/multilingual-e5-base/tokenizer.json `
  --source-id "file://notes"

cargo run --bin search -- "lsm" --limit 10


# 510 = 512 - 2 ([CLS] и [SEP]) — безопасно
$env:HYBRID_CHUNK_MAX_TOKENS = "510"
$env:HYBRID_CHUNK_OVERLAP   = "40"   # как удобно
cargo run --release

$res = Invoke-RestMethod `
   -Uri "http://127.0.0.1:8090/api/ingest/text_raw?title=raw_utf16_ok" `
   -Method POST `
   -ContentType "text/plain" `
   -Body $bytes