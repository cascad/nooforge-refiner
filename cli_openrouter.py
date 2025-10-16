# -*- coding: utf-8 -*-
import argparse
import json
import logging
from pathlib import Path
from typing import Tuple

from logger import setup_logging
from pipeline import run_pipeline


def _tuple_ints(v: str) -> Tuple[int, ...]:
    # позволяет передавать "3,5,7" или повторять --w 3 --w 5
    parts = [p.strip() for p in v.replace(";", ",").split(",") if p.strip()]
    return tuple(int(p) for p in parts)


def main():
    p = argparse.ArgumentParser(description="NooForge semantic segmentation/refine CLI (OpenRouter)")

    # обязательные
    p.add_argument("--model", required=True, help="OpenRouter model id (e.g., qwen/qwen-2.5-72b-instruct)")
    p.add_argument("--input", required=True, help="input text file path")
    p.add_argument("--output", required=True, help="output JSON file path")

    # ---- базовые режимы ----
    p.add_argument("--lang", default="ru", help="document language (ru/en/...)")
    p.add_argument("--seg-mode", default="llm-first", choices=["llm-first", "auto"], help="primary segmentation mode")
    p.add_argument("--sentence-units", action="store_true", help="force 1 sentence = 1 unit")
    p.add_argument("--aggressive", action="store_true", help="more eager cutting/merging")
    p.add_argument("--debug", action="store_true", help="include debug block in output JSON")

    # ---- окна композитов ----
    p.add_argument("--w", dest="windows", type=_tuple_ints, default=(3, 5),
                   help="composite window sizes, e.g. '3,5'")

    p.add_argument("--window-stride", type=int, default=1, help="stride between composite windows")

    # ---- сегментация авто-параметры ----
    p.add_argument("--sim-threshold", type=float, default=None, help="similarity cut threshold")
    p.add_argument("--min-unit-tokens", type=int, default=None, help="min tokens per unit (soft)")
    p.add_argument("--max-unit-tokens", type=int, default=None, help="max tokens per unit (soft)")
    p.add_argument("--seg-target-words", type=int, default=None, help="target words per unit (llm-first)")
    p.add_argument("--seg-min-units", type=int, default=None, help="minimum units to produce")

    # ---- авто-тюнинг перед сегментацией ----
    p.add_argument("--auto-tune", action="store_true", help="enable pre-segmentation auto-tuning")
    p.add_argument("--no-auto-tune", action="store_true", help="disable pre-segmentation auto-tuning")
    p.add_argument("--allow-llm-tune", action="store_true", help="allow LLM to propose segmentation params")
    p.add_argument("--no-allow-llm-tune", action="store_true", help="disallow LLM param tuning")
    p.add_argument("--llm-tune-max-chars", type=int, default=None, help="max chars sampled for LLM tuning")

    # ---- refine/summaries/titles ----
    p.add_argument("--do-llm-refine-units", action="store_true", help="LLM rewrite/clean units ON")
    p.add_argument("--no-llm-refine-units", action="store_true", help="LLM rewrite/clean units OFF")
    p.add_argument("--do-llm-title-rollups", action="store_true", help="LLM titles/rollups ON")
    p.add_argument("--no-llm-title-rollups", action="store_true", help="LLM titles/rollups OFF")
    p.add_argument("--max-summary-words", type=int, default=None, help="summary length target (words)")
    p.add_argument("--title-dedup-window", type=int, default=None, help="anti-repeat window for titles")
    p.add_argument("--no-rollup-anti-repeat", action="store_true", help="disable anti-repeat for rollups")

    # ---- topics (RU/EN) ----
    p.add_argument("--topics-per-unit", type=int, default=None)
    p.add_argument("--topics-per-composite", type=int, default=None)
    p.add_argument("--bilingual-topics", action="store_true", help="produce RU+EN topics")
    p.add_argument("--no-bilingual-topics", action="store_true", help="only RU topics")
    p.add_argument("--topics-en-source", default=None, choices=["llm"], help="how to derive EN topics")
    p.add_argument("--no-translit", action="store_true", help="do NOT add RU->latin transliteration alias")

    # ---- embeddings ----
    p.add_argument("--embed-model", default=None, help="sentence-transformers model (default intfloat/e5-base-v2)")

    # ---- логирование ----
    p.add_argument("--log-level", default="INFO", choices=["DEBUG", "INFO", "WARNING", "ERROR"])
    p.add_argument("--log-json", action="store_true", help="emit logs as JSON lines")

    args = p.parse_args()

    # logging setup
    setup_logging(args.log_level, args.log_json)
    log = logging.getLogger("nooforge_seg.cli")

    # читаем текст
    text = Path(args.input).read_text(encoding="utf-8")

    # собираем kwargs-override для pipeline/config
    overrides = {
        "lang": args.lang,
        "seg_mode": args.seg_mode,
        "aggressive": bool(args.aggressive),
        "sentence_units": bool(args.sentence_units),
        "window_sizes": tuple(args.windows),
        "window_stride": args.window_stride,
    }

    # числовые/флаговые overrides (только если явно заданы)
    if args.sim_threshold is not None: overrides["sim_threshold"] = args.sim_threshold
    if args.min_unit_tokens is not None: overrides["min_unit_tokens"] = args.min_unit_tokens
    if args.max_unit_tokens is not None: overrides["max_unit_tokens"] = args.max_unit_tokens
    if args.seg_target_words is not None: overrides["seg_target_words"] = args.seg_target_words
    if args.seg_min_units is not None: overrides["seg_min_units"] = args.seg_min_units

    # авто-тюнинг
    if args.no_auto_tune: overrides["auto_tune"] = False
    if args.auto_tune: overrides["auto_tune"] = True
    if args.no_allow_llm_tune: overrides["allow_llm_tune"] = False
    if args.allow_llm_tune: overrides["allow_llm_tune"] = True
    if args.llm_tune_max_chars is not None: overrides["llm_tune_max_chars"] = args.llm_tune_max_chars

    # refine/title/rollups
    if args.no_llm_refine_units: overrides["do_llm_refine_units"] = False
    if args.do_llm_refine_units: overrides["do_llm_refine_units"] = True
    if args.no_llm_title_rollups: overrides["do_llm_title_rollups"] = False
    if args.do_llm_title_rollups: overrides["do_llm_title_rollups"] = True
    if args.max_summary_words is not None: overrides["max_summary_words"] = args.max_summary_words
    if args.title_dedup_window is not None: overrides["title_dedup_window"] = args.title_dedup_window
    if args.no_rollup_anti_repeat: overrides["rollup_anti_repeat"] = False

    # topics
    if args.topics_per_unit is not None: overrides["topics_per_unit"] = args.topics_per_unit
    if args.topics_per_composite is not None: overrides["topics_per_composite"] = args.topics_per_composite
    if args.no_bilingual_topics: overrides["bilingual_topics"] = False
    if args.bilingual_topics: overrides["bilingual_topics"] = True
    if args.topics_en_source is not None: overrides["topics_en_source"] = args.topics_en_source
    if args.no_translit: overrides["add_translit_alias"] = False

    # embeddings
    if args.embed_model is not None: overrides["embed_model"] = args.embed_model

    # debug-режим влияет на включение подробных dbg в JSON
    overrides["debug"] = bool(args.debug)

    log.info("run started", extra={"extra": {
        "model": args.model,
        **overrides
    }})

    doc = run_pipeline(
        model=args.model,
        text=text,
        overrides=overrides,
    )

    out_path = Path(args.output)
    out_path.write_text(json.dumps(doc, ensure_ascii=False, indent=2), encoding="utf-8")
    log.info("run finished", extra={"extra": {"output": str(out_path)}})


if __name__ == "__main__":
    main()
