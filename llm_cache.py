# -*- coding: utf-8 -*-
from __future__ import annotations
import hashlib, json, os, sqlite3, threading, time
from typing import Optional, Dict, Any
from llm_iface import LLM

_SCHEMA = """
CREATE TABLE IF NOT EXISTS cache (
  key TEXT PRIMARY KEY,
  ts  REAL NOT NULL,
  model TEXT,
  system TEXT,
  user TEXT,
  params TEXT,
  response TEXT
);
"""

def _mk_key(model: str, system: str, user: str, params: Dict[str, Any]) -> str:
    blob = json.dumps({
        "model": model,
        "system": system,
        "user": user,
        "params": params,
    }, ensure_ascii=False, sort_keys=True).encode("utf-8")
    return hashlib.sha256(blob).hexdigest()

class CachedLLM(LLM):
    """Прозрачный кэш-обёртка над любым LLM (совместимым с LLM.complete)."""
    def __init__(self, base: LLM, path: str = ".nooforge_llm_cache.sqlite", enabled: bool = True):
        self.base = base
        self.path = path
        self.enabled = enabled
        self._lock = threading.Lock()
        if self.enabled:
            self._init()

    def _init(self):
        os.makedirs(os.path.dirname(self.path) or ".", exist_ok=True)
        with sqlite3.connect(self.path) as db:
            db.execute(_SCHEMA)
            db.commit()

    def complete(self, system: str, user: str, max_tokens: int = 512, **kwargs) -> str:
        params = dict(kwargs)
        params["max_tokens"] = max_tokens
        model = getattr(self.base, "model", "unknown")
        key = _mk_key(model, system, user, params)
        if self.enabled:
            with self._lock, sqlite3.connect(self.path) as db:
                cur = db.execute("SELECT response FROM cache WHERE key=?", (key,))
                row = cur.fetchone()
                if row:
                    return row[0]
        # miss
        out = self.base.complete(system=system, user=user, max_tokens=max_tokens, **kwargs)
        if self.enabled:
            with self._lock, sqlite3.connect(self.path) as db:
                db.execute(
                    "INSERT OR REPLACE INTO cache(key, ts, model, system, user, params, response) VALUES (?,?,?,?,?,?,?)",
                    (key, time.time(), model, system, user, json.dumps(params, ensure_ascii=False), out)
                )
                db.commit()
        return out
