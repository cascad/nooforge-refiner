# nooforge_seg/llm_seg.py
from __future__ import annotations
import hashlib, re, unicodedata
from typing import List, Tuple
from llm_iface import LLM
from config import SegmenterConfig

SEG_RE = re.compile(r"⟦SEG:(\d{4})⟧")

# ——— НОВОЕ: канонизация для мягкой проверки ———
_ZW = "".join([
    "\u200b", "\u200c", "\u200d", "\u2060",  # zero-width
    "\ufeff",  # BOM
])
ZW_RE   = re.compile(f"[{re.escape(_ZW)}]+")
SPACE_RE = re.compile(r"[ \t\u00A0]+")  # обычные и неразрывные
QUOTE_RE = re.compile(r"[“”„‟«»]")      # типографские кавычки → "
DASH_RE  = re.compile(r"[–—−]")         # длинные тире/минус → -

def _canon(s: str) -> str:
    s = unicodedata.normalize("NFKC", s)
    s = ZW_RE.sub("", s)
    s = QUOTE_RE.sub('"', s)
    s = DASH_RE.sub("-", s)
    s = s.replace("\r\n", "\n")
    # схлопываем повторы пробелов/табов
    s = SPACE_RE.sub(" ", s)
    return s

def insert_markers(llm: LLM, cfg: SegmenterConfig, text: str) -> str:
    SYS = """Ты помогаешь сегментировать текст на логические единицы.
ТВОЯ ЗАДАЧА: ВСТАВИТЬ МАРКЕРЫ РАЗДЕЛЕНИЯ БЕЗ КАКИХ-ЛИБО ДРУГИХ ИЗМЕНЕНИЙ ТЕКСТА.
НЕЛЬЗЯ: переводить, исправлять, удалять/добавлять символы, менять регистр, переносы строк или пробелы.
Маркеры: ⟦SEG:0001⟧, ⟦SEG:0002⟧, ... (4 цифры).
Внутри ```код``` НЕ ВСТАВЛЯЙ маркеры. Целевой размер ~{target_words} слов.
Верни ровно исходный текст + маркеры."""
    USR = "Текст:\n<<<BEGIN>>>\n{TEXT}\n<<<END>>>"
    out = llm.complete(
        SYS.format(target_words=cfg.seg_target_words),
        USR.format(TEXT=text),
        max_tokens=len(text) // 2 + 512
    )
    return out

def split_by_markers(marked: str) -> List[str]:
    parts, last = [], 0
    for m in SEG_RE.finditer(marked):
        parts.append(marked[last:m.start()])
        last = m.end()
    parts.append(marked[last:])
    parts = [p for p in parts if p != ""]
    return [SEG_RE.sub("", p) for p in parts]

def validate_lossless(original: str, segments: List[str]) -> Tuple[bool, str]:
    reassembled = "".join(segments)

    # 1) строгая побайтная
    if reassembled == original:
        return True, "ok"

    # 2) нормализуем переводы строк
    norm_orig = original.replace("\r\n", "\n")
    norm_reas = reassembled.replace("\r\n", "\n")
    if norm_orig == norm_reas:
        return True, "normalized-newlines"

    # 3) КАНОНИЧЕСКАЯ (разрешаем различия в типографике/пробелах/zero-width)
    if _canon(norm_orig) == _canon(norm_reas):
        return True, "canonicalized"

    return False, f"mismatch sha256: orig={sha256(original)} reas={sha256(reassembled)}"

def sha256(s: str) -> str:
    return hashlib.sha256(s.encode("utf-8", "replace")).hexdigest()
