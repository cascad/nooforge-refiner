#!/usr/bin/env python3
import sys
from pathlib import Path
from typing import List
import time

from llama_index.core import Document, VectorStoreIndex, StorageContext
from llama_index.core.node_parser import SemanticSplitterNodeParser
from llama_index.embeddings.huggingface import HuggingFaceEmbedding
from llama_index.vector_stores.qdrant import QdrantVectorStore
from qdrant_client import QdrantClient

import config

try:
    # новые версии qdrant-client
    from qdrant_client.http.exceptions import UnexpectedResponse  # type: ignore
except Exception:
    try:
        # вдруг твоя версия кладёт сюда
        from qdrant_client.exceptions import UnexpectedResponse  # type: ignore
    except Exception:
        # fallback: маркер, что специфичного класса нет
        UnexpectedResponse = None  # type: ignore


class DocumentIngestion:
    def __init__(self):
        print("Initializing ingestion pipeline...")

        # Setup embedding model
        print(f"Loading embeddings: {config.EMBED_MODEL}")
        self.embed_model = HuggingFaceEmbedding(
            model_name=config.EMBED_MODEL,
            cache_folder=str(config.CACHE_DIR / "embeddings")
        )

        # Setup semantic splitter
        print("Initializing semantic splitter...")
        self.splitter = SemanticSplitterNodeParser(
            buffer_size=1,
            breakpoint_percentile_threshold=config.SEMANTIC_THRESHOLD,
            embed_model=self.embed_model,
        )

        # Setup vector store
        print(
            f"Connecting to Qdrant: {config.QDRANT_HOST}:{config.QDRANT_PORT}")
        self.client = QdrantClient(
            host=config.QDRANT_HOST,
            port=config.QDRANT_PORT,
            timeout=config.QDRANT_TIMEOUT_SEC,
            prefer_grpc=config.QDRANT_PREFER_GRPC,      # по умолчанию False
            check_compatibility=config.QDRANT_CHECK_COMPAT,  # по умолчанию False
        )

        # Wait for Qdrant to be ready
        max_retries = 20
        print("Waiting for Qdrant to be ready...")
        for i in range(max_retries):
            try:
                r = self.client.get_collections()  # REST вызов
                cnt = getattr(r, "collections", []) or []
                print(f"✓ Qdrant is ready (collections: {len(cnt)})")
                break
            except UnexpectedResponse as e:
                # Печатаем побольше деталей — часто тут 503 из-за прокси/IPv6/gRPC
                status = getattr(e, "status_code", "n/a")
                body = getattr(e, "response", b"")
                try:
                    body = body.decode() if isinstance(body, (bytes, bytearray)) else str(body)
                except Exception:
                    body = str(body)
                print(f"  Attempt {i+1}/{max_retries}... (UnexpectedResponse {status}) body={body!r}")
            except Exception as e:
                print(f"  Attempt {i+1}/{max_retries}... ({e.__class__.__name__}: {e})")
            time.sleep(2)
        else:
            raise RuntimeError("Qdrant not responding after retries; check client flags, IPv4, and NO_PROXY.")

        self.vector_store = QdrantVectorStore(
            client=self.client,
            collection_name=config.COLLECTION_NAME,
        )

        print("✓ Ready\n")

    def load_documents(self, data_dir: Path) -> List[Document]:
        """Load documents from directory"""
        documents = []

        # Supported extensions
        extensions = [".txt", ".md", ".py", ".js", ".rs"]

        files = []
        for ext in extensions:
            files.extend(data_dir.rglob(f"*{ext}"))

        print(f"Found {len(files)} files")

        for file_path in files:
            try:
                text = file_path.read_text(encoding='utf-8')

                doc = Document(
                    text=text,
                    metadata={
                        'filename': file_path.name,
                        'filepath': str(file_path),
                        'extension': file_path.suffix,
                    }
                )

                documents.append(doc)
                print(f"  ✓ {file_path.name} ({len(text)} chars)")

            except Exception as e:
                print(f"  ✗ {file_path.name}: {e}")

        return documents

    def ingest(self, data_dir: Path):
        """Full ingestion pipeline"""
        print(f"\n{'='*60}")
        print("DOCUMENT INGESTION")
        print(f"{'='*60}\n")

        # Load documents
        documents = self.load_documents(data_dir)
        if not documents:
            print("No documents found!")
            return

        print(f"\nLoaded {len(documents)} documents")

        # Create storage context
        storage_context = StorageContext.from_defaults(
            vector_store=self.vector_store
        )

        # Build index with semantic chunking
        print("\nBuilding index (this may take a while)...")
        start = time.time()

        index = VectorStoreIndex.from_documents(
            documents,
            storage_context=storage_context,
            embed_model=self.embed_model,
            transformations=[self.splitter],
            show_progress=True,
        )

        elapsed = time.time() - start

        print(f"\n✅ Index built in {elapsed:.1f}s")
        print(f"Collection: {config.COLLECTION_NAME}")

        # Stats
        collection_info = self.client.get_collection(config.COLLECTION_NAME)
        print(f"Vectors: {collection_info.points_count}")


def main():
    import argparse

    parser = argparse.ArgumentParser(
        description="Ingest documents into vector store")
    parser.add_argument("--data", default="data", help="Data directory")
    parser.add_argument("--reset", action="store_true",
                        help="Reset collection")
    args = parser.parse_args()

    data_dir = Path(args.data)
    if not data_dir.exists():
        print(f"Data directory not found: {data_dir}")
        sys.exit(1)

    ingestion = DocumentIngestion()

    # Reset collection if requested
    if args.reset:
        print("Resetting collection...")
        try:
            ingestion.client.delete_collection(config.COLLECTION_NAME)
            print("✓ Collection deleted\n")
        except:
            pass

    ingestion.ingest(data_dir)


if __name__ == "__main__":
    main()
