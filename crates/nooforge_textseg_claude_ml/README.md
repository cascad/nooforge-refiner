# Укажи свой API-ключ OpenRouter
$env:OPENROUTER_API_KEY = "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"

# Настройки модели (можно менять)
$env:OPENROUTER_MODEL = "qwen/qwen-2.5-72b-instruct:free"

# Параметры оконной сегментации
$env:SEG_WIN = "4096"      # размер окна в байтах
$env:SEG_OVERLAP = "256"   # перекрытие между окнами
$env:SEG_RETRIES = "2"     # повтор при ошибке
$env:SMART_CHUNK = "4096"
$env:SMART_CHUNK = "2048"

# Запуск в release-режиме
cargo run --release -- ai_article.txt
cargo run --release --bin nooforge-textseg-claude-openrouter -- ai_article.txt

----

$env:OPENROUTER_API_KEY = "sk-..."     # ключ
$env:OPENROUTER_MODEL   = "qwen/qwen-2.5-72b-instruct:free"
$env:SEG_VERBOSE        = "1"          # чтобы видеть 🌐/✅ логи по каждой итерации
$env:SEG_WIN            = "2048"       # 4096
$env:SEG_OVERLAP        = "256"
$env:SEG_RETRIES        = "2"
$env:SEG_K              = "3"          # self-consistency k
$env:SEG_MINLEN         = "12"         # анти-коротыши

cargo run --release --bin main -- .\mixed_torture_test.txt