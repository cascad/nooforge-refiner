# file: query_openrouter_minimal.py
import os
from llama_index.llms.openrouter import OpenRouter
from llama_index.core import Settings

from dotenv import load_dotenv
load_dotenv()

# --- конфиг ---
# os.environ["OPENROUTER_API_KEY"] = "sk-or-..."   # твой ключ
os.environ["OPENROUTER_BASE_URL"] = "https://openrouter.ai/api/v1"

# --- создаём LLM напрямую ---
llm = OpenRouter(
    api_key=os.environ["OPENROUTER_API_KEY"],
    model="anthropic/claude-3.5-sonnet",
    base_url=os.environ["OPENROUTER_BASE_URL"],
    default_headers={
        "HTTP-Referer": "http://localhost",
        "X-Title": "nooforge-refiner",
    },
)

Settings.llm = llm

# --- проверка ---
resp = llm.complete("Скажи 'ок' по-русски.")
print("✅ Ответ:", resp.text)
