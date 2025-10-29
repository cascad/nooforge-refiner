import os
from pathlib import Path
from dotenv import load_dotenv

load_dotenv()

# Paths
PROJECT_ROOT = Path(__file__).parent
DATA_DIR = PROJECT_ROOT / "data"
CACHE_DIR = PROJECT_ROOT / ".cache"

# Vector DB
QDRANT_HOST = os.getenv("QDRANT_HOST", "127.0.0.1")
QDRANT_PORT = int(os.getenv("QDRANT_PORT", 6333))
COLLECTION_NAME = os.getenv("COLLECTION_NAME", "documents")

# Новые флаги (по умолчанию безопасные)
QDRANT_CHECK_COMPAT = os.getenv("QDRANT_CHECK_COMPAT", "false").lower() == "true"
QDRANT_PREFER_GRPC   = os.getenv("QDRANT_PREFER_GRPC", "false").lower() == "true"
QDRANT_TIMEOUT_SEC  = int(os.getenv("QDRANT_TIMEOUT_SEC", "60"))

# Embeddings
EMBED_MODEL = os.getenv("EMBED_MODEL", "sentence-transformers/all-MiniLM-L6-v2")
# For Russian: "intfloat/multilingual-e5-small"

# LLM
LLM_PROVIDER = os.getenv("LLM_PROVIDER", "openrouter")

# OpenRouter
OPENROUTER_API_KEY = os.getenv("OPENROUTER_API_KEY", "")
OPENROUTER_MODEL = os.getenv("OPENROUTER_MODEL", "anthropic/claude-sonnet-4.5")
OPENROUTER_BASE_URL = "https://openrouter.ai/api/v1"
OPENROUTER_HTTP_REFERER= os.getenv("OPENROUTER_HTTP_REFERER", "http://localhost").strip()  # укажи свой домен/репо, если есть
OPENROUTER_X_TITLE     = os.getenv("OPENROUTER_X_TITLE", "nooforge-refiner").strip()

# OpenAI (fallback)
OPENAI_API_KEY = os.getenv("OPENAI_API_KEY", "")
OPENAI_MODEL = os.getenv("OPENAI_MODEL", "gpt-4")

# Chunking
CHUNK_SIZE = int(os.getenv("CHUNK_SIZE", 512))
CHUNK_OVERLAP = int(os.getenv("CHUNK_OVERLAP", 50))
SEMANTIC_THRESHOLD = int(os.getenv("SEMANTIC_THRESHOLD", 95))

# Search
TOP_K = int(os.getenv("TOP_K", 5))