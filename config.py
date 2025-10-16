# -*- coding: utf-8 -*-
from __future__ import annotations
from dataclasses import dataclass, field
from typing import Tuple, Optional, Dict, Any

@dataclass
class SegmenterConfig:
    # язык + базовые режимы
    lang: str = "ru"
    seg_mode: str = "llm-first"  # "llm-first" | "auto"
    sentence_units: bool = False
    aggressive: bool = False

    # авто-параметры сегментации
    sim_threshold: float = 0.575
    min_unit_tokens: int = 48
    max_unit_tokens: int = 232
    seg_target_words: int = 70
    seg_min_units: int = 2
    auto_tune: bool = False
    allow_llm_tune: bool = False
    llm_tune_max_chars: Optional[int] = None

    # окна композитов (если comp_mode="window")
    window_sizes: Tuple[int, ...] = (3, 5)
    window_stride: int = 1

    # композиты (иерархический режим)
    comp_mode: str = "hier"  # "hier" | "window"
    cluster_sim_threshold: float = 0.82  # порог косинуса для присоединения кластера
    cluster_target_count: int = 8        # желаемое число кластеров (мягкая цель)
    comp_dedup_sim: float = 0.92         # дедуп похожих композитов по rollup-эмбеддингам

    # LLM-постобработка
    do_llm_refine_units: bool = True
    do_llm_title_rollups: bool = True
    max_summary_words: Optional[int] = 60
    title_dedup_window: int = 5
    rollup_anti_repeat: bool = True

    # темы (topics)
    topics_per_unit: int = 4
    topics_per_composite: int = 6
    bilingual_topics: bool = False       # включить EN-темы
    topics_en_source: Optional[str] = "llm"  # пока только "llm"
    add_translit_alias: bool = True

    # эмбеддинги
    embed_model: str = "intfloat/e5-base-v2"

    # fused режимы
    fuse_units: bool = True
    fuse_composites: bool = True

    # кэширование
    enable_cache: bool = True
    cache_path: str = ".nooforge_llm_cache.sqlite"

    # отладка
    debug: bool = False

    @classmethod
    def from_overrides(cls, overrides: Dict[str, Any] | None) -> "SegmenterConfig":
        cfg = cls()
        if overrides:
            for k, v in overrides.items():
                if hasattr(cfg, k):
                    setattr(cfg, k, v)
        return cfg
