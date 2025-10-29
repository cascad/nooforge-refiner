# file: llamaindex_full/query.py
#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import os
import sys
import argparse
from typing import List, Optional

# 0) .env
try:
    from dotenv import load_dotenv
    load_dotenv(".env")
except Exception:
    pass

# Прибиваем прокси и IPv6-неожиданности для локала
os.environ["NO_PROXY"] = "127.0.0.1,localhost"
os.environ["no_proxy"] = "127.0.0.1,localhost"
for _k in ("HTTP_PROXY", "http_proxy", "HTTPS_PROXY", "https_proxy"):
    os.environ.pop(_k, None)

# 1) Конфиг
try:
    import config
except Exception as e:
    print("FATAL: cannot import config.py:", e, file=sys.stderr)
    sys.exit(1)

OPENROUTER_API_KEY      = (getattr(config, "OPENROUTER_API_KEY", None) or os.getenv("OPENROUTER_API_KEY", "")).strip()
OPENROUTER_BASE_URL     = (getattr(config, "OPENROUTER_BASE_URL", None) or os.getenv("OPENROUTER_BASE_URL", "https://openrouter.ai/api/v1")).strip()
OPENROUTER_HTTP_REFERER = (getattr(config, "OPENROUTER_HTTP_REFERER", None) or os.getenv("OPENROUTER_HTTP_REFERER", "http://localhost")).strip()
OPENROUTER_X_TITLE      = (getattr(config, "OPENROUTER_X_TITLE", None) or os.getenv("OPENROUTER_X_TITLE", "nooforge-refiner")).strip()
OPENROUTER_MODEL        = (getattr(config, "OPENROUTER_MODEL", None) or os.getenv("OPENROUTER_MODEL", "anthropic/claude-3.5-sonnet")).strip()

QDRANT_HOST             = (getattr(config, "QDRANT_HOST", None) or os.getenv("QDRANT_HOST", "127.0.0.1")).strip()
QDRANT_PORT             = int(getattr(config, "QDRANT_PORT", None) or os.getenv("QDRANT_PORT", "6333"))
QDRANT_CHECK_COMPAT     = bool(getattr(config, "QDRANT_CHECK_COMPAT", False) or os.getenv("QDRANT_CHECK_COMPAT", "").lower() == "true")
QDRANT_PREFER_GRPC      = bool(getattr(config, "QDRANT_PREFER_GRPC", False) or os.getenv("QDRANT_PREFER_GRPC", "").lower() == "true")
QDRANT_TIMEOUT_SEC      = int(getattr(config, "QDRANT_TIMEOUT_SEC", 60) or os.getenv("QDRANT_TIMEOUT_SEC", "60"))
COLLECTION_NAME         = (getattr(config, "COLLECTION_NAME", None) or os.getenv("COLLECTION_NAME", "nooforge_docs")).strip()
TOP_K_DEFAULT           = int(getattr(config, "TOP_K", 5) or os.getenv("TOP_K", "5"))

# Подстелим соломку: любой код OpenAI SDK пусть тоже смотрит на OpenRouter
os.environ["OPENAI_API_KEY"]  = OPENROUTER_API_KEY
os.environ["OPENAI_BASE_URL"] = OPENROUTER_BASE_URL
os.environ["HTTP-Referer"]    = OPENROUTER_HTTP_REFERER
os.environ["X-Title"]         = OPENROUTER_X_TITLE

# 2) LlamaIndex / Qdrant импорты
from llama_index.core import VectorStoreIndex, Settings
from llama_index.core.vector_stores import VectorStoreQuery
from llama_index.core.query_engine import RetrieverQueryEngine
from llama_index.vector_stores.qdrant import QdrantVectorStore
from llama_index.embeddings.huggingface import HuggingFaceEmbedding
from qdrant_client import QdrantClient

# response synthesizer (совместимость по версиям)
try:
    from llama_index.core.response_synthesizers import get_response_synthesizer, ResponseMode
    _RESP_MODE = getattr(ResponseMode, "COMPACT", "compact")
except Exception:
    from llama_index.core.response_synthesizers import get_response_synthesizer  # type: ignore
    _RESP_MODE = "compact"


def _build_llm():
    """
    Создаём LLM, которая гарантированно бьёт в OpenRouter.
    1) Пытаемся использовать нативный провайдер LlamaIndex: llama_index.llms.openrouter.OpenRouter (без OpenAI SDK).
    2) Если его нет — используем OpenAILike с base_url/api_base и sanity-проверкой.
    """
    if not OPENROUTER_API_KEY:
        raise RuntimeError("OPENROUTER_API_KEY не задан. Положи ключ в .env или config.py")

    llm = None
    used = ""

    # Попытка №1: нативный провайдер OpenRouter (предпочтительно)
    try:
        from llama_index.llms.openrouter import OpenRouter as LIOpenRouter  # type: ignore
        llm = LIOpenRouter(
            api_key=OPENROUTER_API_KEY,
            model=OPENROUTER_MODEL,
            max_tokens=None,  # не ограничиваем искусственно
            default_headers={
                "HTTP-Referer": OPENROUTER_HTTP_REFERER,
                "X-Title": OPENROUTER_X_TITLE,
            },
            timeout=90,
            base_url=OPENROUTER_BASE_URL,  # если поддерживается
        )
        used = "LlamaIndex.OpenRouter"
    except Exception:
        # Попытка №2: OpenAILike (OpenAI-совместимый клиент, но с ручным base_url)
        try:
            from llama_index.llms.openai_like import OpenAILike  # type: ignore
        except Exception as e:
            print("FATAL: neither llama_index.llms.openrouter.OpenRouter nor openai_like.OpenAILike is available:", e, file=sys.stderr)
            sys.exit(1)

        # сначала пробуем современную сигнатуру
        try:
            llm = OpenAILike(
                model=OPENROUTER_MODEL,
                base_url=OPENROUTER_BASE_URL,
                api_key=OPENROUTER_API_KEY,
                timeout=90,
                additional_headers={
                    "HTTP-Referer": OPENROUTER_HTTP_REFERER,
                    "X-Title": OPENROUTER_X_TITLE,
                },
            )
            used = "OpenAILike(base_url=...)"
        except TypeError:
            # fallback: старая сигнатура
            llm = OpenAILike(
                model=OPENROUTER_MODEL,
                api_base=OPENROUTER_BASE_URL,
                api_key=OPENROUTER_API_KEY,
                timeout=90,
                additional_headers={
                    "HTTP-Referer": OPENROUTER_HTTP_REFERER,
                    "X-Title": OPENROUTER_X_TITLE,
                },
            )
            used = "OpenAILike(api_base=...)"

    Settings.llm = llm

    # Sanity: попробуем достать «видимый» base_url у используемого LLM (если есть такое поле)
    try:
        bu = getattr(llm, "base_url", None) or getattr(llm, "api_base", None)
        print(f"LLM provider: {used}; base_url={bu or '<unknown>'}")
    except Exception:
        print(f"LLM provider: {used}; base_url=<unknown>")

    return llm


def _build_embed():
    embed = HuggingFaceEmbedding(model_name="intfloat/multilingual-e5-small")
    Settings.embed_model = embed
    return embed


def _build_qdrant_store():
    client = QdrantClient(
        host=QDRANT_HOST,
        port=QDRANT_PORT,
        timeout=QDRANT_TIMEOUT_SEC,
        prefer_grpc=QDRANT_PREFER_GRPC,
        check_compatibility=QDRANT_CHECK_COMPAT,
    )
    return QdrantVectorStore(
        client=client,
        collection_name=COLLECTION_NAME,
    )


class RAGQuery:
    def __init__(self, top_k: int):
        print("Initializing query engine...")
        self.llm = _build_llm()
        print(f"Using OpenRouter model: {OPENROUTER_MODEL}")

        self.embed = _build_embed()
        vstore = _build_qdrant_store()

        self.index = VectorStoreIndex.from_vector_store(
            vstore,
            embed_model=self.embed,
        )

        self.retriever = self.index.as_retriever(similarity_top_k=top_k)
        self.synth = get_response_synthesizer(
            response_mode=_RESP_MODE,
            llm=Settings.llm,
        )

        self.query_engine = RetrieverQueryEngine(
            retriever=self.retriever,
            response_synthesizer=self.synth,
        )
        print("✓ Ready")

    @staticmethod
    def _pretty_sources(nodes: List) -> str:
        out = []
        for i, n in enumerate(nodes, 1):
            meta = getattr(n, "metadata", {}) or {}
            score = getattr(n, "score", None)
            doc_id = meta.get("document_id") or meta.get("doc_id") or meta.get("id") or "-"
            path  = meta.get("source") or meta.get("file_path") or meta.get("filepath") or "-"
            title = meta.get("title") or meta.get("filename") or "-"
            if score is None and hasattr(n, "score") and n.score is not None:
                score = n.score
            out.append(f"[{i}] score={score!r} id={doc_id} title={title} src={path}")
        return "\n".join(out)

    def search_only(self, query_text: str, top_k: Optional[int] = None):
        if top_k is not None:
            self.retriever.similarity_top_k = top_k
        vs: QdrantVectorStore = self.index._vector_store  # type: ignore
        q = VectorStoreQuery(query_str=query_text, similarity_top_k=self.retriever.similarity_top_k)
        res = vs.query(q)
        print(f"\nTop-{self.retriever.similarity_top_k} results for: {query_text!r}")
        for i, (pt, sc) in enumerate(zip(res.nodes, res.similarities or []), 1):
            meta = getattr(pt, "metadata", {}) or {}
            title = meta.get("title") or meta.get("filename") or "-"
            src   = meta.get("source") or meta.get("file_path") or meta.get("filepath") or "-"
            print(f"[{i}] score={sc:.4f} title={title!r} src={src}")

    def query(self, query_text: str):
        print(f"Query: {query_text}")
        resp = self.query_engine.query(query_text)
        print("\n=== ANSWER ===\n")
        print(str(resp).strip())
        if getattr(resp, "source_nodes", None):
            print("\n=== SOURCES ===")
            print(self._pretty_sources(resp.source_nodes))


def _build_argparser():
    p = argparse.ArgumentParser(description="RAG via Qdrant + OpenRouter (LlamaIndex)")
    p.add_argument("query", nargs="?", help="Текст запроса")
    p.add_argument("--interactive", "-i", action="store_true", help="Интерактивный режим")
    p.add_argument("--search-only", action="store_true", help="Только топ из векторки (без LLM ответа)")
    p.add_argument("--top-k", type=int, default=TOP_K_DEFAULT, help=f"Сколько документов забирать (default: {TOP_K_DEFAULT})")
    return p


def main():
    parser = _build_argparser()
    args = parser.parse_args()

    print(f"OpenRouter base_url = {OPENROUTER_BASE_URL}")
    if not OPENROUTER_API_KEY:
        print("WARNING: OPENROUTER_API_KEY пуст — будет 401.", file=sys.stderr)

    # Sanity: какой base_url видит «сырой» OpenAI SDK (на случай скрытых путей)
    try:
        from openai import OpenAI as RawOpenAI
        raw = RawOpenAI(api_key=OPENROUTER_API_KEY, base_url=OPENROUTER_BASE_URL)
        print("Sanity OpenAI base_url:", raw.base_url)
    except Exception as e:
        print("Sanity OpenAI base_url check failed:", e)

    if not args.query and not args.interactive:
        parser.print_help()
        sys.exit(1)

    rag = RAGQuery(top_k=args.top_k)

    if args.interactive:
        print("Введите запросы (Ctrl+C чтобы выйти):")
        try:
            while True:
                q = input("> ").strip()
                if not q:
                    continue
                if args.search_only:
                    rag.search_only(q, top_k=args.top_k)
                else:
                    rag.query(q)
        except KeyboardInterrupt:
            print("\nBye!")
    else:
        if args.search_only:
            rag.search_only(args.query, top_k=args.top_k)
        else:
            rag.query(args.query)


if __name__ == "__main__":
    # Чиним вывод UTF-8 на Windows при необходимости
    try:
        if sys.platform.startswith("win") and (sys.stdout.encoding or "").lower() != "utf-8":
            import io, msvcrt
            msvcrt.setmode(sys.stdout.fileno(), os.O_BINARY)  # type: ignore
            sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8")
    except Exception:
        pass
    main()
