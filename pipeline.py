# -*- coding: utf-8 -*-
from __future__ import annotations
import logging
from typing import Dict, Any

from sentence_transformers import SentenceTransformer

from config import SegmenterConfig
from segmenter import SemanticSegmenter
from composer import build_composites
from refine import refine_units_fused, refine_units, build_document_topics
from llm_openrouter import OpenRouterLLM
from llm_cache import CachedLLM

log = logging.getLogger("nooforge_seg.pipeline")

def run_pipeline(
    model: str,
    text: str,
    overrides: Dict[str, Any] | None = None,
) -> Dict[str, Any]:
    cfg = SegmenterConfig.from_overrides(overrides or {})

    # LLM + cache
    base_llm = OpenRouterLLM(model=model)
    llm = CachedLLM(base_llm, path=cfg.cache_path, enabled=bool(cfg.enable_cache))

    # единая модель эмбеддингов (переиспользуем и в сегментере, и в композитах)
    embed = SentenceTransformer(cfg.embed_model)

    # segmenter: передаём llm и embed в конструктор; segment() принимает только text
    seg = SemanticSegmenter(cfg=cfg, llm=llm, embed_model=embed)
    units, seg_dbg = seg.segment(text)

    # refine units (fused or legacy)
    if cfg.do_llm_refine_units:
        if cfg.fuse_units:
            units = refine_units_fused(llm, cfg, units)
        else:
            units = refine_units(llm, cfg, units)

    # composites (hier by default; inside uses fused composite)
    composites, comp_dbg = build_composites(llm, cfg, units, embed)

    # document summary (на базе первого композита либо первых юнитов)
    if composites:
        doc_summary = composites[0]["rollup"]
    elif units:
        doc_summary = " ".join(u.get("summary") or u["text"] for u in units[:2])
    else:
        doc_summary = ""

    # document topics
    doc_topics = build_document_topics(llm, cfg, units, per=cfg.topics_per_composite)

    out: Dict[str, Any] = {
        "document": {
            "summary": doc_summary,
            "units": units,
            "composites": composites,
        }
    }
    if cfg.debug:
        out["document"]["debug"] = {
            "tuned_params": {
                "sim_threshold": cfg.sim_threshold,
                "min_unit_tokens": cfg.min_unit_tokens,
                "max_unit_tokens": cfg.max_unit_tokens,
            },
            "aggressive": bool(cfg.aggressive),
            "sentence_units": bool(cfg.sentence_units),
            "segmentation": seg_dbg,
            "composites": comp_dbg,
        }

    return out
