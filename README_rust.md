# NooForge Refiner — Rust


## Сборка
```
cargo build --release
```


## Промпты
- Все промпты вынесены в `crates/prompts/prompts/` (см. ниже). По умолчанию они **вшиты** в бинарь (через `rust-embed`).
- Для override на диске установите `NOOFORGE_PROMPTS_DIR=/path/to/prompts`. Горячая замена поддерживается на уровне «прочитал при запуске».


Структура:
```
crates/prompts/prompts/
composite/system.txt
composite/user.txt
refine/system.txt
refine/user.txt
```
Шаблоны используют плейсхолдеры `{{TEXT}}`, `{{BILINGUAL}}` и т.п.


## Примеры
```
OPENROUTER_API_KEY=sk-... \
cargo run -p refiner-cli -- \
--model qwen/qwen-2.5-72b-instruct \
--input samples/tst04.txt \
--output out.json \
--seg-mode llm-first \
--sentence-units \
--cache .cache/nooforge_llm \
--log-level debug
```


## Соответствие Python-версии
- segmenter.py → refiner-core/src/segmenter.rs
- refine.py (refine_units, build_composite) → refiner-core/src/refine.rs
- composer.py → refiner-core/src/composer.rs
- pipeline.py → refiner-core/src/pipeline.rs
- llm_openrouter.py / llm_iface.py → llm-traits + llm-openrouter
- llm_cache.py → crates/cache
- **Промпты** → crates/prompts


## TODO
- Логирование + метрики на запросы
- Добавить локальные бэкенды (`ollama`, `llama.cpp`/`vllm`) за фичами.
- Метрики (prometheus exporter) и трейсинг span’ами.
- Тонкая сегментация (абзацы/заголовки/буллеты) + «fused» композиты.
- Золотые эталоны JSON и property-тесты на стабильность схемы.


```sh
$env:OPENROUTER_API_KEY = "sk-..." 
cargo run -p refiner-cli --release -- `
  --model qwen/qwen-2.5-72b-instruct `
  --input samples/in.txt `
  --output out.json `
  --seg-mode llm-first `
  --sentence-units `
  --cache .cache/nooforge_llm `
  --log-level info
```

```sh
$env:OPENROUTER_API_KEY = "sk-..." 
cargo run -p refiner-cli --release -- `
  --model qwen/qwen-2.5-72b-instruct `
  --input samples/tst04.txt `
  --output out.json `
  --seg-mode llm-first `
  --sentence-units `
  --cache .cache/nooforge_llm `
  --log-level info
```

```sh
$env:OPENROUTER_API_KEY = "sk-..." 
cargo run -p refiner-cli --release -- `
  --model qwen/qwen-2.5-72b-instruct `
  --input samples/postgres_parts.txt `
  --output out.json `
  --seg-mode llm-first `
  --sentence-units `
  --cache .cache/nooforge_llm `
  --log-level info
```