# -*- coding: utf-8 -*-
from __future__ import annotations
from typing import List, Dict, Tuple
import numpy as np

from sentence_transformers import SentenceTransformer
from config import SegmenterConfig
from refine import (
    fused_composite,         # 1 запрос: title+rollup+topics
    title_for_block,         # fallback
    rollup_for_window,       # fallback
    extract_topics_ru,       # fallback
)

def _cos(a: np.ndarray, b: np.ndarray) -> float:
    if a.ndim == 1: a = a[None, :]
    if b.ndim == 1: b = b[None, :]
    denom = (np.linalg.norm(a, axis=1) * np.linalg.norm(b, axis=1)) + 1e-9
    return float(np.dot(a, b.T) / denom)

def _greedy_clusters(vectors: np.ndarray, sim_thr: float) -> List[List[int]]:
    """Простой жадный кластеризатор: идём слева направо, присоединяем к ближайшему
    кластеру, у которого косинус с центроидом >= sim_thr, иначе создаём новый."""
    clusters: List[List[int]] = []
    centroids: List[np.ndarray] = []
    for i in range(vectors.shape[0]):
        v = vectors[i]
        best, best_sim = -1, -1.0
        for c_idx, c in enumerate(clusters):
            cen = centroids[c_idx]
            s = _cos(v, cen)
            if s > best_sim:
                best_sim, best = s, c_idx
        if best >= 0 and best_sim >= sim_thr:
            clusters[best].append(i)
            # обновим центроид
            arr = np.vstack([vectors[j] for j in clusters[best]])
            centroids[best] = arr.mean(axis=0)
        else:
            clusters.append([i])
            centroids.append(v.copy())
    return clusters

def _limit_clusters(clusters: List[List[int]], target: int) -> List[List[int]]:
    if target is None or target <= 0 or len(clusters) <= target:
        return clusters
    # просто оставим крупнейшие
    clusters = sorted(clusters, key=lambda c: (-len(c), c[0]))
    return clusters[:target]

def _dedup_composites(rollup_embs: np.ndarray, comps: List[Dict], sim_thr: float) -> Tuple[List[Dict], np.ndarray]:
    keep_idx = []
    for i in range(len(comps)):
        vi = rollup_embs[i]
        dup = False
        for j in keep_idx:
            if _cos(vi, rollup_embs[j]) >= sim_thr:
                dup = True; break
        if not dup:
            keep_idx.append(i)
    comps2 = [comps[i] for i in keep_idx]
    embs2 = np.vstack([rollup_embs[i] for i in keep_idx]) if keep_idx else np.zeros((0, rollup_embs.shape[1]))
    return comps2, embs2

def build_composites(
    llm,
    cfg: SegmenterConfig,
    units: List[Dict],
    embed_model: SentenceTransformer
) -> Tuple[List[Dict], Dict]:
    dbg = {"windows": [], "fail_safe_used": False}
    composites: List[Dict] = []
    if not units:
        dbg["composites_count"] = 0
        return composites, dbg

    texts = [u.get("refined") or u["text"] for u in units]
    # эмбеддинги юнитов — по summary, если есть
    unit_text_for_emb = [u.get("summary") or (u.get("refined") or u["text"]) for u in units]
    U = embed_model.encode(unit_text_for_emb, normalize_embeddings=True)
    n = len(units)

    if cfg.comp_mode == "hier":
        clusters = _greedy_clusters(U, cfg.cluster_sim_threshold)
        clusters = _limit_clusters(clusters, cfg.cluster_target_count)
        for idx, cl in enumerate(clusters):
            block_units = [units[i] for i in cl]
            block_texts = [texts[i] for i in cl]
            fused = fused_composite(llm, cfg, block_texts)
            comp = {
                "id": f"comp_hier_{idx:04d}",
                "type": "composite_hier",
                "units": [u["id"] for u in block_units],
                "title": fused.get("title", ""),
                "rollup": fused.get("rollup", ""),
                "topics": fused.get("topics", []),
                "topics_en": fused.get("topics_en", []) if cfg.bilingual_topics else [],
                "span": [block_units[0].get("start"), block_units[-1].get("end")],
            }
            composites.append(comp)
        dbg["windows"].append({"w": "hier", "count": len(composites), "stride": None})

    else:
        # window mode (с фьюжном)
        seen = set()
        recent_titles: List[str] = []
        for w in cfg.window_sizes:
            w_eff = min(w, n)
            if w_eff < 2:
                continue
            made = 0
            for start in range(0, max(0, n - w_eff + 1), cfg.window_stride):
                end = start + w_eff
                key = (w_eff, start, end)
                if key in seen: continue
                seen.add(key)

                block_units = units[start:end]
                block_texts = texts[start:end]
                fused = fused_composite(llm, cfg, block_texts)

                comp = {
                    "id": f"comp_w{w_eff}_{start:04d}_{end:04d}",
                    "type": f"composite_window:{w_eff}",
                    "units": [u["id"] for u in block_units],
                    "title": fused.get("title", ""),
                    "rollup": fused.get("rollup", ""),
                    "topics": fused.get("topics", []),
                    "topics_en": fused.get("topics_en", []) if cfg.bilingual_topics else [],
                    "span": [block_units[0].get("start"), block_units[-1].get("end")],
                }
                composites.append(comp); made += 1
                if comp["title"]:
                    recent_titles.append(comp["title"])
            dbg["windows"].append({"w": w_eff, "count": made, "stride": cfg.window_stride})

    # эмбеддинги композитов по rollup (для дедупа)
    comp_rollups = [c["rollup"] for c in composites]
    if comp_rollups:
        C = embed_model.encode(comp_rollups, normalize_embeddings=True)
        if cfg.comp_dedup_sim and len(composites) > 1:
            composites, C = _dedup_composites(C, composites, cfg.comp_dedup_sim)

    dbg["composites_count"] = len(composites)
    return composites, dbg
