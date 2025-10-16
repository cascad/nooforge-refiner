# -*- coding: utf-8 -*-
from typing import List, Dict, Tuple, Any
from sentence_transformers import SentenceTransformer
import numpy as np, re

from config import SegmenterConfig
from structure import split_structural_blocks, punctuation_density, pseudo_sentence_windows
from llm_seg import insert_markers, split_by_markers, validate_lossless
from llm_iface import LLM

# Сплит по предложениям — с позициями
_SENT_SPLIT = re.compile(r'(?<=[.!?…])\s+(?=[A-ZА-Я0-9\(])', flags=re.U)


def split_sentences_with_spans(text: str) -> List[Dict]:
    """Вернёт [{'text':..., 'start':i, 'end':j}, ...] c исходными позициями."""
    if not text.strip():
        return []
    parts = []
    last = 0
    for m in _SENT_SPLIT.finditer(text):
        seg = text[last:m.start()]
        if seg.strip():
            parts.append({"text": seg, "start": last, "end": m.start()})
        last = m.end()
    tail = text[last:]
    if tail.strip():
        parts.append({"text": tail, "start": last, "end": last + len(tail)})
    if not parts:
        parts = [{"text": text, "start": 0, "end": len(text)}]
    return parts


def token_len(s: str) -> int:
    return len(s.split())


class SemanticSegmenter:
    """
    Канонический интерфейс:
      - __init__(cfg, llm=None, embed_model=None)
      - segment(text) -> (units, debug_dict)

    Где:
      * cfg: SegmenterConfig
      * llm: LLM | None
      * embed_model: SentenceTransformer | None (если None — создадим по cfg.embed_model)
    """

    def __init__(self, cfg: SegmenterConfig, llm: LLM | None = None, embed_model: SentenceTransformer | None = None):
        self.cfg = cfg
        self.llm = llm
        # если embed_model не передали — поднимем локально
        self.embed_model = embed_model or SentenceTransformer(cfg.embed_model)

    # --- helpers ---

    def _emb(self, sentences: List[str]) -> np.ndarray:
        return self.embed_model.encode(sentences, normalize_embeddings=True, convert_to_numpy=True)

    def _join_text(self, chunks: List[Dict]) -> str:
        return " ".join(x["text"].strip() for x in chunks).strip()

    # --- основные шаги сегментации ---

    def _segment_block_auto(self, block: str, dbg: Dict) -> List[Dict]:
        target_len = 60 if self.cfg.aggressive else 80
        stride     = 35 if self.cfg.aggressive else 50

        if self.cfg.sentence_units:
            sents = split_sentences_with_spans(block)
            dbg["mode"] = "sentence-units"
            return [{"text": s["text"], "start": s["start"], "end": s["end"]} for s in sents]

        if punctuation_density(block) >= 0.01:
            sents = split_sentences_with_spans(block)
            dbg["mode"] = "auto/punct"
            base_texts = [s["text"].strip() for s in sents]
        else:
            base_texts = pseudo_sentence_windows(block, target_len=target_len, stride=stride, max_len=self.cfg.max_unit_tokens)
            sents = [{"text": t, "start": None, "end": None} for t in base_texts]
            dbg["mode"] = "auto/pseudo"

        if len(sents) == 1:
            dbg["pairs"] = 0
            return sents

        embs = self._emb([s["text"] for s in sents])
        units: List[List[Dict]] = []
        cur: List[Dict] = [sents[0]]
        cuts = []

        for i in range(1, len(sents)):
            sim = float(np.dot(embs[i-1], embs[i]))
            cur_text = " ".join(x["text"] for x in cur + [sents[i]])
            over = token_len(cur_text) > self.cfg.max_unit_tokens
            do_cut = (sim < self.cfg.sim_threshold) or over
            cuts.append({"i": i, "sim": sim, "cut": do_cut, "over": over})
            if do_cut:
                if units and token_len(" ".join(x["text"] for x in cur)) < self.cfg.min_unit_tokens:
                    units[-1].extend(cur)
                else:
                    units.append(cur)
                cur = [sents[i]]
            else:
                cur.append(sents[i])

        if cur:
            if units and token_len(" ".join(x["text"] for x in cur)) < self.cfg.min_unit_tokens:
                units[-1].extend(cur)
            else:
                units.append(cur)

        dbg["pairs"] = len(sents) - 1
        dbg["cuts"]  = cuts

        out: List[Dict] = []
        for grp in units:
            if all(x["start"] is not None for x in grp):
                start = grp[0]["start"]; end = grp[-1]["end"]
                out.append({"text": self._join_text(grp), "start": start, "end": end})
            else:
                text = self._join_text(grp)
                out.append({"text": text, "start": None, "end": None})
        return out

    def _fallback_min_units(self, block_text: str, want: int, dbg: Dict) -> List[Dict]:
        """Если после автосегментации 1 юнит — режем по предложениям (агрессивно: 1 предложение = 1 юнит)."""
        sents = split_sentences_with_spans(block_text)
        if len(sents) >= 2:
            if self.cfg.aggressive or self.cfg.sentence_units:
                out = [{"text": s["text"], "start": s["start"], "end": s["end"]} for s in sents]
                dbg["fallback"] = f"sentence-force:{len(out)}"
                return out
            else:
                out: List[Dict] = []
                cur: List[Dict] = []
                for s in sents:
                    cur.append(s)
                    if token_len(self._join_text(cur)) >= max(1, self.cfg.min_unit_tokens // 2):
                        out.append({"text": self._join_text(cur), "start": cur[0]["start"], "end": cur[-1]["end"]})
                        cur = []
                if cur:
                    if out:
                        out[-1]["text"] = (out[-1]["text"] + " " + self._join_text(cur)).strip()
                        out[-1]["end"] = cur[-1]["end"]
                    else:
                        out = cur
                dbg["fallback"] = f"sentence-pack:{len(out)}"
                return out

        # Одно «предложение» — режем грубо по словам
        words = block_text.split()
        if len(words) < 80:
            dbg["fallback"] = "none"
            return [{"text": block_text, "start": None, "end": None}]
        parts = 3 if (self.cfg.aggressive or self.cfg.sentence_units) else (2 if len(words) < 180 else 3)
        size = len(words) // parts
        out = []
        for i in range(parts):
            a = i * size
            b = len(words) if i == parts - 1 else (i + 1) * size
            t = " ".join(words[a:b]).strip()
            out.append({"text": t, "start": None, "end": None})
        dbg["fallback"] = f"word-split:{len(out)}"
        return out

    # --- публичный API (канон) ---

    def segment(self, text: str) -> Tuple[List[Dict], Dict]:
        blocks = split_structural_blocks(text)
        debug_info: Dict[str, Any] = {"blocks": []}
        units_all: List[Dict] = []
        offset = 0  # смещение блока в документе

        for b in blocks:
            b_dbg: Dict[str, Any] = {"len": len(b), "mode": None, "llm_markers": 0, "lossless": None, "fallback": None}
            block_units: List[Dict] = []

            if self.cfg.seg_mode == "llm-first" and self.llm is not None and b.strip() and not self.cfg.sentence_units:
                try:
                    marked = insert_markers(self.llm, self.cfg, b)
                    parts = split_by_markers(marked)
                    b_dbg["llm_markers"] = max(0, len(parts) - 1)
                    ok, why = validate_lossless(b, parts)
                    b_dbg["lossless"] = why
                    if ok and len(parts) >= self.cfg.seg_min_units:
                        b_dbg["mode"] = "llm-first"
                        block_units = []
                        cursor = 0
                        for t in parts:
                            idx = b.find(t, cursor)
                            if idx >= 0:
                                block_units.append({"text": t, "start": idx, "end": idx + len(t)})
                                cursor = idx + len(t)
                            else:
                                block_units.append({"text": t, "start": None, "end": None})
                    else:
                        block_units = self._segment_block_auto(b, b_dbg)
                except Exception as e:
                    b_dbg["mode"] = f"llm-error:{type(e).__name__}"
                    block_units = self._segment_block_auto(b, b_dbg)
            else:
                block_units = self._segment_block_auto(b, b_dbg)

            if len(block_units) < 2 and (self.cfg.aggressive or self.cfg.sentence_units):
                block_units = self._fallback_min_units(b, want=4, dbg=b_dbg)

            # перенос спанов блока в координаты документа
            for u in block_units:
                if u["start"] is not None and u["end"] is not None:
                    units_all.append({
                        "id": None,
                        "text": u["text"],
                        "start": offset + u["start"],
                        "end":   offset + u["end"]
                    })
                else:
                    units_all.append({"id": None, "text": u["text"], "start": None, "end": None})

            debug_info["blocks"].append(b_dbg)
            offset += len(b) + 1  # +1 за разделительную пустую строку

        if not units_all:
            units_all = [{"text": text.strip(), "start": 0, "end": len(text.strip())}] if text.strip() else []

        # финальная разметка id — аккуратно, чтобы None не перетёр готовый id
        out = []
        for i, u in enumerate(units_all, 1):
            d = dict(u)
            d.pop("id", None)
            d["id"] = f"unit_{i:04d}"
            out.append(d)

        debug_info["units_count"] = len(out)
        return out, debug_info
