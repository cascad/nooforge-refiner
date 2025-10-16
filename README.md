# NooForge Refiner

Модуль для смысловой сегментации и обогащения текста через LLM (локально или в облаке).  
Используется для построения документных структур, сводок и тематических аннотаций.

---

### ⚙️ Текущие флаги CLI

| Флаг | Описание |
|------|-----------|
| `--model` | имя модели (например, `qwen/qwen-2.5-72b-instruct`) |
| `--input` | путь к входному текстовому файлу |
| `--output` | путь для сохранения результата в JSON |
| `--seg-mode` | режим сегментации (`auto`, `llm-first`, `sentences-only`) |
| `--sentence-units` | разбивать текст на юниты по предложениям |
| `--bilingual-topics` | извлекать темы и переводить их на английский |
| `--w` | размеры окон композита (через запятую, например `3,5`) |
| `--log-level` | уровень логирования (`INFO`, `DEBUG`, `ERROR`) |
| `--strategy` | стратегия выполнения (`local`, `remote`, `hybrid`) |
| `--cache` | путь к каталогу для кэша (по умолчанию `.cache/nooforge_llm`) |
| `--no-cache` | отключить кэширование вызовов LLM |
| `--embed-model` | модель эмбеддингов (по умолчанию `sentence-transformers/all-MiniLM-L6-v2`) |
| `--refine` | включить постобработку юнитов через LLM |
| `--fuse` | использовать fused-композиты (title + rollup + topics) |
| `--debug` | включить расширенный вывод отладки |
| `--max-tokens` | ограничение на длину ответов LLM |
| `--temperature` | температура генерации (по умолчанию 0.1) |

---

### 🪟 Пример (Windows PowerShell)
```sh
python -m cli_openrouter `
  --model qwen/qwen-2.5-72b-instruct `
  --input samples/tst04.txt `
  --output out.json `
  --seg-mode llm-first `
  --sentence-units `
  --bilingual-topics `
  --w 3,5 `
  --log-level DEBUG
```

```sh
python -m cli_openrouter `
  --model qwen/qwen-2.5-72b-instruct `
  --input samples/postgres_parts.txt `
  --output out.json `
  --seg-mode llm-first `
  --sentence-units `
  --bilingual-topics `
  --w 3,5 `
  --log-level DEBUG
```
