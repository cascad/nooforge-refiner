# -*- coding: utf-8 -*-
from __future__ import annotations
import logging
import re
import json
from typing import List, Dict, Any, Iterable, Optional

from config import SegmenterConfig
from llm_iface import LLM

log = logging.getLogger("nooforge_seg.refine")

# ============================== helpers ==============================

_WS = re.compile(r"\s+")
_SENT_END = re.compile(r"[.!?…]$")

# закрывающие символы, которые могут стоять после финальной точки
_WRAPPERS = set(')"\']»›）】〉』」»”’ ]')

def _norm(s: str) -> str:
    return _WS.sub(" ", (s or "").strip()).strip()

def _to_key(s: str) -> str:
    return _norm(s).lower()

def _unique_keep_order(items: Iterable[str]) -> List[str]:
    seen = set()
    out: List[str] = []
    for x in items:
        k = _to_key(x)
        if k in seen:
            continue
        seen.add(k)
        out.append(_norm(x))
    return out

def _drop_substrings(phrases: List[str], min_len: int = 3) -> List[str]:
    normed = [_norm(p) for p in phrases if len(_norm(p)) >= min_len]
    keep: List[str] = []
    for i, p in enumerate(normed):
        kp = _to_key(p)
        longer = False
        for j, q in enumerate(normed):
            if i == j:
                continue
            if len(q) > len(p) and kp in _to_key(q):
                longer = True
                break
        if not longer:
            keep.append(p)
    return keep

_STOP = {
    "и","в","на","по","из","для","к","от","как","при","что",
    "of","the","to","in","a","an","for","on","and","or"
}

def _clean_topic_tokens(s: str) -> str:
    parts = [t for t in re.split(r"[ /,;|]+", s) if t]
    parts = [t for t in parts if _to_key(t) not in _STOP]
    return " ".join(parts) if parts else _norm(s)

def _truncate_words(s: str, max_words: Optional[int]) -> str:
    if not max_words or max_words <= 0:
        return _norm(s)
    words = s.split()
    if len(words) <= max_words:
        return _norm(s)
    return _norm(" ".join(words[:max_words]).rstrip(",.;:") + "…")

def _ensure_sentence_end_dot(s: str) -> str:
    """Нормализует хвост предложения: ... → …, точка перед закрывающими скобками/кавычками, без удвоений."""
    s = (s or "").strip()
    if not s:
        return s
    # сводим "..." → "…"
    s = re.sub(r"\.\.\.$", "…", s)

    # отделим хвост из закрывающих символов
    suffix: List[str] = []
    while s and s[-1] in _WRAPPERS:
        suffix.append(s[-1])
        s = s[:-1].rstrip()

    # если нет завершающей пунктуации — добавим точку
    if not _SENT_END.search(s):
        s = s + "."

    # вернём суффикс обратно
    if suffix:
        s = s + "".join(reversed(suffix))
    return s

# ============================== LLM I/O ==============================

def _llm_json_array(llm: LLM, system: str, user: str, max_tokens: int = 256) -> List[str]:
    resp = llm.complete(system=system, user=user, max_tokens=max_tokens)
    try:
        arr = json.loads(resp)
        if isinstance(arr, list) and all(isinstance(x, str) for x in arr):
            return arr
    except Exception as e:
        log.warning("llm_json_array_parse_error", extra={"extra": {"err": type(e).__name__, "resp": resp[:400]}})
    return []

def _llm_text(llm: LLM, system: str, user: str, max_tokens: int = 256) -> str:
    try:
        out = llm.complete(system=system, user=user, max_tokens=max_tokens)
        return _norm(out)
    except Exception as e:
        log.warning("llm_text_error", extra={"extra": {"err": type(e).__name__}})
        return ""

# ============================== prompts ==============================

# RU refine
_SYS_REFINE = (
    "You rewrite Russian technical text into a clean, concise Russian version. "
    "Keep meaning, fix minor grammar, preserve terms. Output plain text only."
)
_USER_REFINE = "Перепиши кратко и ясно (русский), без потери смысла. Текст:\n{t}"

# RU summary
_SYS_SUMMARY = "You write a one-sentence Russian summary (max ~25–30 words). Output plain text only."
_USER_SUMMARY = "Сделай одно предложение-резюме (25–30 слов максимум) по этому тексту (русский):\n{text}"

# RU topics extract
_SYS_TOPICS_RU = (
    "Ты извлекаешь ключевые темы (теги) из текста на русском языке. "
    "Отвечай строго JSON-массивом коротких фраз (2–5 слов), без комментариев."
)
_USER_TOPICS_RU = "Извлеки до {limit} ключевых тем (2–5 слов) в формате JSON-массива. Текст:\n{t}"

# EN topics translate (полноценный перевод, без транслита)
_SYSTEM_TOPICS_EN = (
    "You translate topic phrases into natural, concise English suitable for search tags. "
    "Keep domain acronyms (e.g., WAL, MVCC) and proper nouns as canonical English. "
    "Return ONLY a JSON array of strings."
)
_USER_TRANSLATE_TEMPLATE = """Translate these Russian topic phrases to natural English search tags.
- Each phrase should be standalone, human English (no transliteration).
- Keep technical acronyms and proper nouns as is.
- Prefer concise forms (2–5 words). Return ONLY JSON array.

Phrases:
{lines}
"""

# RU title
_SYS_TITLE = (
    "You create a short Russian title (3–6 words) for a technical snippet. "
    "Be specific and concise. Output plain text only, no quotes."
)
_USER_TITLE = "Сформулируй короткий заголовок (3–6 слов, русский) по этому фрагменту:\n{text}"

# RU rollup (2–3 sentences)
_SYS_ROLLUP = (
    "You write a compact Russian roll-up (2–3 sentences) that synthesizes the given units. "
    "Be factual, avoid repetition, keep key mechanisms and relationships. Output plain text only."
)
_USER_ROLLUP = "Синтезируй краткий обзор (2–3 предложения, русский) по этим фрагментам:\n{joined}"

# ============================== topics logic ==============================

def _extract_topics_ru(llm: LLM, cfg: SegmenterConfig, text: str, limit: int) -> List[str]:
    user = _USER_TOPICS_RU.format(limit=limit or 5, t=text)
    arr = _llm_json_array(llm, _SYS_TOPICS_RU, user, max_tokens=256)
    if not arr:
        # минимальный фолбэк — возьмём несколько содержательных токенов
        words = [w for w in re.findall(r"[A-Za-zА-Яа-я0-9\-]+", text) if len(w) > 3]
        candidates: List[str] = []
        if words:
            chunks = [" ".join(words[i:i+3]) for i in range(0, min(len(words), 12), 3)]
            candidates = [c for c in chunks if c]
        arr = candidates or ["темы", "обзор"]
    arr = [_clean_topic_tokens(x) for x in arr]
    arr = _unique_keep_order(arr)
    arr = _drop_substrings(arr)
    if limit and len(arr) > limit:
        arr = arr[:limit]
    return arr

def _latin_translit_soft(s: str) -> str:
    tbl = str.maketrans({
        "а":"a","б":"b","в":"v","г":"g","д":"d","е":"e","ё":"e","ж":"zh","з":"z",
        "и":"i","й":"i","к":"k","л":"l","м":"m","н":"n","о":"o","п":"p","р":"r",
        "с":"s","т":"t","у":"u","ф":"f","х":"h","ц":"ts","ч":"ch","ш":"sh","щ":"shch",
        "ы":"y","э":"e","ю":"yu","я":"ya","ь":"","ъ":"",
        "А":"a","Б":"b","В":"v","Г":"g","Д":"d","Е":"e","Ё":"e","Ж":"zh","З":"z",
        "И":"i","Й":"i","К":"k","Л":"l","М":"m","Н":"n","О":"o","П":"p","Р":"r",
        "С":"s","Т":"t","У":"u","Ф":"f","Х":"h","Ц":"ts","Ч":"ch","Ш":"sh","Щ":"shch",
        "Ы":"y","Э":"e","Ю":"yu","Я":"ya",
    })
    return s.translate(tbl)

def _derive_topics_en_with_llm(llm: LLM, ru_topics: List[str], limit: Optional[int]) -> List[str]:
    if not ru_topics:
        return []
    lines = "\n".join(f"- {t}" for t in ru_topics)
    user = _USER_TRANSLATE_TEMPLATE.format(lines=lines)
    arr = _llm_json_array(llm, _SYSTEM_TOPICS_EN, user, max_tokens=300)
    if not arr:
        return []
    arr = [_norm(x) for x in arr]
    arr = _unique_keep_order(arr)
    arr = _drop_substrings(arr, min_len=3)
    if limit and len(arr) > limit:
        arr = arr[:limit]
    return arr

def _derive_topics_en_fallback(ru_topics: List[str], limit: Optional[int]) -> List[str]:
    out = []
    for t in ru_topics:
        tr = _latin_translit_soft(t)
        tr = _clean_topic_tokens(tr)
        out.append(tr)
    out = _unique_keep_order(out)
    out = _drop_substrings(out)
    if limit and len(out) > limit:
        out = out[:limit]
    return out

def augment_bilingual_topics(llm: LLM, cfg: SegmenterConfig, text_or_rollup: str, ru_topics: List[str]) -> Dict[str, List[str]]:
    """Возвращает {"topics": RU, "topics_en": EN}, где EN — через LLM, фолбэк — транслит."""
    limit = getattr(cfg, "topics_per_unit", None) or getattr(cfg, "topics_per_composite", None) or 5
    en = _derive_topics_en_with_llm(llm, ru_topics, limit=limit)
    if not en:
        en = _derive_topics_en_fallback(ru_topics, limit=limit)
    return {"topics": ru_topics, "topics_en": en}

# ============================== unit refine ==============================

def _refine_single(llm: LLM, text: str) -> Dict[str, str]:
    refined = _llm_text(llm, _SYS_REFINE, _USER_REFINE.format(t=text), max_tokens=256) or text
    refined = _ensure_sentence_end_dot(refined)
    summary = _llm_text(llm, _SYS_SUMMARY, _USER_SUMMARY.format(text=refined), max_tokens=128) or refined
    summary = _ensure_sentence_end_dot(summary)
    return {"refined": refined, "summary": summary}

def refine_units(llm: LLM, cfg: SegmenterConfig, units: List[Dict]) -> List[Dict]:
    """BACK-COMPAT: тонкая обёртка над batch-версией."""
    return refine_units_fused(llm, cfg, units)

def refine_units_fused(llm: LLM, cfg: SegmenterConfig, units: List[Dict]) -> List[Dict]:
    """
    Экономный режим: для каждой единицы — 2 LLM-вызова (refine + summary).
    Плюс topics (RU) и опционально EN (LLM → транслит фолбэк).
    """
    if not units:
        return []

    max_ru = cfg.topics_per_unit or 4
    bilingual = bool(getattr(cfg, "bilingual_topics", False))

    for u in units:
        text = u.get("refined") or u.get("text") or ""
        rs = _refine_single(llm, text)
        u["refined"] = rs["refined"]
        u["summary"] = rs["summary"]

        # topics (ru)
        topics_ru = _extract_topics_ru(llm, cfg, u["refined"], limit=max_ru)
        topics_ru = _drop_substrings(_unique_keep_order(topics_ru))
        if max_ru and len(topics_ru) > max_ru:
            topics_ru = topics_ru[:max_ru]
        u["topics"] = topics_ru

        # topics_en (опционально)
        if bilingual:
            en = _derive_topics_en_with_llm(llm, topics_ru, limit=max_ru)
            if not en:
                en = _derive_topics_en_fallback(topics_ru, limit=max_ru)
            u["topics_en"] = _unique_keep_order(_drop_substrings([_norm(x) for x in en]))

    return units

# ============================== titles / rollups ==============================

def title_for_block(llm: LLM, cfg: SegmenterConfig, text: str, avoid_titles: Optional[List[str]] = None) -> str:
    title = _llm_text(llm, _SYS_TITLE, _USER_TITLE.format(text=text), max_tokens=64)
    if not title:
        title = _truncate_words(text.strip(), 6)
    title = _norm(title)
    # анти-повтор
    window = getattr(cfg, "title_dedup_window", 5) or 5
    if avoid_titles:
        k = _to_key(title)
        for t in avoid_titles[-window:]:
            if k == _to_key(t) or k in _to_key(t) or _to_key(t) in k:
                extra = _truncate_words(text.strip(), 2)
                if extra and extra.lower() not in title.lower():
                    title = _norm(f"{title} — {extra}")
                break
    return title

def rollup_for_window(llm: LLM, cfg: SegmenterConfig, unit_texts: List[str]) -> str:
    joined = "\n".join(f"- {t}" for t in unit_texts if t.strip())
    roll = _llm_text(llm, _SYS_ROLLUP, _USER_ROLLUP.format(joined=joined), max_tokens=300)
    if not roll:
        sk = " ".join(unit_texts[:2]).strip()
        roll = _truncate_words(sk, getattr(cfg, "max_summary_words", 60) or 60)
    # пост-обрезка по словам
    roll = _truncate_words(roll, getattr(cfg, "max_summary_words", None))
    return _ensure_sentence_end_dot(roll)

# ============================== document topics ==============================

def build_document_topics(llm: LLM, cfg: SegmenterConfig, items: List[Dict], per: int) -> List[str]:
    """
    Сводные темы по документу. Если включён bilingual_topics — возвращаем EN темы,
    иначе — RU. Собираем из items[*].topics.
    """
    bag: List[str] = []
    for it in items:
        bag.extend(it.get("topics", []))
    bag = _unique_keep_order(bag)
    bag = _drop_substrings(bag)
    if per and len(bag) > per:
        bag = bag[:per]

    if getattr(cfg, "bilingual_topics", False):
        en = _derive_topics_en_with_llm(llm, bag, limit=per)
        if not en:
            en = _derive_topics_en_fallback(bag, limit=per)
        en = _unique_keep_order(_drop_substrings(en))
        return en

    return bag

# ============================== fused composite ==============================

def fused_composite(
    llm: LLM,
    cfg: SegmenterConfig,
    unit_texts: List[str],
    avoid_titles: Optional[List[str]] = None,
    recent_rollups: Optional[List[str]] = None,
) -> Dict[str, Any]:
    """
    Универсальный «склейщик» для composer: по списку текстов делает title/rollup и темы.
    Возвращает: {"title", "rollup", "topics", "topics_en"} (topics_en только при bilingual_topics).
    """
    title = title_for_block(llm, cfg, "\n".join(unit_texts), avoid_titles=avoid_titles)
    rollup = rollup_for_window(llm, cfg, unit_texts)
    topics_ru = _extract_topics_ru(llm, cfg, rollup, getattr(cfg, "topics_per_composite", 5) or 5)

    if getattr(cfg, "bilingual_topics", False):
        bi = augment_bilingual_topics(llm, cfg, rollup, topics_ru)
        topics_ru, topics_en = bi["topics"], bi["topics_en"]
    else:
        topics_en = []

    return {
        "title": title,
        "rollup": rollup,
        "topics": topics_ru,
        "topics_en": topics_en,
    }

# ============================== back-compat aliases ==============================

def make_rollup(llm: LLM, cfg: SegmenterConfig, text_or_unit_texts: Any, sentences: int = 3,
                recent_titles: Optional[List[str]] = None,
                recent_rollups: Optional[List[str]] = None) -> str:
    """Старая сигнатура — поддержка."""
    if isinstance(text_or_unit_texts, str):
        unit_texts = [text_or_unit_texts]
    elif isinstance(text_or_unit_texts, list):
        unit_texts = text_or_unit_texts
    else:
        unit_texts = [str(text_or_unit_texts)]
    return rollup_for_window(llm, cfg, unit_texts)

def make_title(llm: LLM, cfg: SegmenterConfig, text: str, recent_titles: Optional[List[str]] = None) -> str:
    return title_for_block(llm, cfg, text, avoid_titles=recent_titles)

def extract_topics_ru(llm: LLM, cfg: SegmenterConfig, text: str, limit: int) -> List[str]:
    return _extract_topics_ru(llm, cfg, text, limit)

def derive_topics_en(llm: LLM, ru_topics: List[str], limit: Optional[int] = None) -> List[str]:
    return _derive_topics_en_with_llm(llm, ru_topics, limit)

def derive_topics_en_fallback(ru_topics: List[str], limit: Optional[int] = None) -> List[str]:
    return _derive_topics_en_fallback(ru_topics, limit)
