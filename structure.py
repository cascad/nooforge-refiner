# nooforge_seg/structure.py
import re
from typing import List

RE_FENCE_OPEN = re.compile(r"^\s*```")
RE_HEADING    = re.compile(r"^\s*#{1,6}\s+\S")
RE_HR         = re.compile(r"^\s*(-{3,}|_{3,}|\*{3,})\s*$")
RE_LIST       = re.compile(r"^\s*([\-*•+]|(\d+[\.)-]))\s+")
RE_CHECK      = re.compile(r"^\s*[-*]\s+\[[ xX]\]\s+")
RE_CODE_INDENT= re.compile(r"^\s{4,}\S")

_PUNCT = set(list(".,!?…:;"))

def punctuation_density(text: str) -> float:
    words = max(1, len(text.split()))
    punct = sum(1 for ch in text if ch in _PUNCT)
    return punct / words

def split_structural_blocks(text: str) -> List[str]:
    lines = text.replace("\r\n", "\n").split("\n")
    blocks: List[str] = []
    buf: List[str] = []
    in_fence = False

    def flush():
        nonlocal buf
        if buf and any(s.strip() for s in buf):
            blocks.append("\n".join(buf).strip())
        buf = []

    for ln in lines:
        if RE_FENCE_OPEN.match(ln):
            if in_fence:
                buf.append(ln)
                flush()
                in_fence = False
            else:
                flush()
                in_fence = True
                buf = [ln]
            continue

        if in_fence:
            buf.append(ln)
            continue

        if not ln.strip():
            flush()
            continue

        if RE_HEADING.match(ln) or RE_HR.match(ln) or RE_LIST.match(ln) or RE_CHECK.match(ln) or RE_CODE_INDENT.match(ln):
            flush()
            blocks.append(ln.strip())
            continue

        buf.append(ln)

    flush()
    return blocks

def pseudo_sentence_windows(text: str, target_len: int = 80, stride: int = 50, max_len: int = 220) -> List[str]:
    words = text.split()
    if len(words) <= target_len:
        return [text.strip()]

    chunks = []
    i = 0
    while i < len(words):
        j = min(len(words), i + target_len)
        chunks.append(" ".join(words[i:j]).strip())
        if j >= len(words):
            break
        i += stride

    merged: List[str] = []
    for frag in chunks:
        if not merged:
            merged.append(frag); continue
        if len(merged[-1].split()) < target_len//2:
            merged[-1] = (merged[-1] + " " + frag).strip()
        else:
            merged.append(frag)

    out = []
    for frag in merged:
        if len(frag.split()) > max_len:
            w = frag.split()
            mid = len(w)//2
            out.append(" ".join(w[:mid]))
            out.append(" ".join(w[mid:]))
        else:
            out.append(frag)
    return [s for s in out if s]
