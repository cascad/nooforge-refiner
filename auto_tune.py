from __future__ import annotations
import json
import re
from dataclasses import dataclass
from typing import Optional, Dict, Any
from .config import SegmenterConfig
from .llm_iface import LLM

@dataclass
class AutoTuneResult:
    sim_threshold: Optional[float] = None
    min_unit_tokens: Optional[int] = None
    max_unit_tokens: Optional[int] = None

def _heuristics(text: str) -> AutoTuneResult:
    words = text.split()
    w_count = len(words)
    punct = sum(ch in ".,;:!?…" for ch in text)
    punct_ratio = punct / max(1, len(text))  # на символ
    lines = text.replace("\r\n", "\n").split("\n")
    avg_line_len = sum(len(l) for l in lines) / max(1, len(lines))

    if punct_ratio > 0.015:
        sim = 0.65
    elif punct_ratio > 0.007:
        sim = 0.60
    else:
        sim = 0.55

    min_unit = 40 if avg_line_len < 80 else 70

    if w_count < 2000:
        max_unit = 260
    elif w_count < 6000:
        max_unit = 320
    else:
        max_unit = 380

    return AutoTuneResult(sim_threshold=sim, min_unit_tokens=min_unit, max_unit_tokens=max_unit)

def _parse_llm_json(s: str) -> Optional[Dict[str, Any]]:
    m = re.search(r"\{.*\}", s, re.S)
    if not m:
        return None
    try:
        return json.loads(m.group(0))
    except Exception:
        return None

def _llm_recommend(llm: LLM, cfg: SegmenterConfig, text: str) -> AutoTuneResult:
    snippet = text[: cfg.llm_tune_max_chars]
    system = (
        "Ты настраиваешь параметры сегментации документа на логические единицы. "
        "Оцени текст и предложи численные параметры в JSON без комментариев. "
        "Ключи: sim_threshold (0.5–0.7), min_unit_tokens (20–120), max_unit_tokens (120–600). "
        "Не добавляй ничего, кроме JSON."
    )
    user = (
        f"Текст (фрагмент):\n---\n{snippet}\n---\n"
        "Верни JSON, например: {\"sim_threshold\":0.6,\"min_unit_tokens\":60,\"max_unit_tokens\":320}"
    )
    try:
        out = llm.complete(system, user, max_tokens=180).strip()
        obj = _parse_llm_json(out)
        if not obj:
            return AutoTuneResult()
        sim = obj.get("sim_threshold")
        mn = obj.get("min_unit_tokens")
        mx = obj.get("max_unit_tokens")

        def clamp(v, lo, hi):
            return max(lo, min(hi, v))

        if isinstance(sim, (int, float)):
            sim = clamp(float(sim), 0.5, 0.7)
        else:
            sim = None

        if isinstance(mn, (int, float)):
            mn = int(clamp(int(mn), 20, 120))
        else:
            mn = None

        if isinstance(mx, (int, float)):
            mx = int(clamp(int(mx), 120, 600))
        else:
            mx = None

        return AutoTuneResult(sim_threshold=sim, min_unit_tokens=mn, max_unit_tokens=mx)
    except Exception:
        return AutoTuneResult()

def adaptive_tune(cfg: SegmenterConfig, text: str, llm: Optional[LLM]):
    base = _heuristics(text)

    sim = base.sim_threshold if base.sim_threshold is not None else cfg.sim_threshold
    mn  = base.min_unit_tokens if base.min_unit_tokens is not None else cfg.min_unit_tokens
    mx  = base.max_unit_tokens if base.max_unit_tokens is not None else cfg.max_unit_tokens

    if llm is not None and cfg.allow_llm_tune:
        lr = _llm_recommend(llm, cfg, text)
        if lr.sim_threshold is not None:
            sim = (sim + lr.sim_threshold) / 2.0
        if lr.min_unit_tokens is not None:
            mn = int((mn + lr.min_unit_tokens) / 2)
        if lr.max_unit_tokens is not None:
            mx = int((mx + lr.max_unit_tokens) / 2)

    # Агрессивный режим: сильнее дробим (ниже порог и меньше размеры юнитов)
    if cfg.aggressive:
        # sim = max(0.50, sim - 0.05)         
        sim = max(0.48, sim - 0.07)         # ↓ порог → больше разрезов
        mn  = max(20, int(mn * 0.75))       # ↓ мин. размер
        mx  = max(140, int(mx * 0.80))      # ↓ макс. размер
        # LLM целевой размер логической единицы тоже уменьшим
        cfg.seg_target_words = max(70, int(cfg.seg_target_words * 0.75))

    cfg.sim_threshold   = float(sim)
    cfg.min_unit_tokens = int(mn)
    cfg.max_unit_tokens = int(mx)
    return cfg, {"sim_threshold": cfg.sim_threshold, "min_unit_tokens": cfg.min_unit_tokens, "max_unit_tokens": cfg.max_unit_tokens}
