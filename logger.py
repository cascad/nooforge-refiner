# -*- coding: utf-8 -*-
import logging
import json
import time
from contextlib import contextmanager

class _JsonFormatter(logging.Formatter):
    def format(self, record: logging.LogRecord) -> str:
        payload = {
            "level": record.levelname,
            "time": self.formatTime(record, datefmt="%Y-%m-%dT%H:%M:%S"),
            "logger": record.name,
            "msg": record.getMessage(),
        }
        if record.exc_info:
            payload["exc_info"] = self.formatException(record.exc_info)
        if hasattr(record, "extra") and isinstance(record.extra, dict):
            payload.update(record.extra)  # type: ignore
        return json.dumps(payload, ensure_ascii=False)

def setup_logging(level: str = "INFO", json_mode: bool = False) -> None:
    """
    level: DEBUG | INFO | WARNING | ERROR
    json_mode: если True — выводит JSON-строки (удобно парсить).
    """
    lvl = getattr(logging, level.upper(), logging.INFO)
    root = logging.getLogger()
    root.handlers.clear()
    root.setLevel(lvl)

    handler = logging.StreamHandler()
    if json_mode:
        handler.setFormatter(_JsonFormatter())
    else:
        fmt = "%(asctime)s | %(levelname)s | %(name)s | %(message)s"
        datefmt = "%H:%M:%S"
        handler.setFormatter(logging.Formatter(fmt=fmt, datefmt=datefmt))
    root.addHandler(handler)

@contextmanager
def timed(logger: logging.Logger, name: str, **kv):
    """
    Контекст-таймер:
    with timed(log, "refine_units", units=len(units)):
        ...
    """
    t0 = time.perf_counter()
    logger.debug(f"⏱ start {name}", extra={"extra": kv})
    try:
        yield
    finally:
        dt = (time.perf_counter() - t0) * 1000.0
        kv2 = dict(kv); kv2["ms"] = round(dt, 2)
        logger.debug(f"✅ done {name}", extra={"extra": kv2})
