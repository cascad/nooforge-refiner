import os, requests
from llm_iface import LLM

OPENROUTER_URL = "https://openrouter.ai/api/v1/chat/completions"

class OpenRouterLLM(LLM):
    def __init__(self, model: str, api_key: str | None = None, extra_headers: dict | None = None,
                 temperature: float = 0.0, top_p: float = 1.0):
        self.model = model
        self.api_key = api_key or os.getenv("OPENROUTER_API_KEY")
        if not self.api_key:
            raise RuntimeError("Set OPENROUTER_API_KEY env var")
        self.headers = {"Authorization": f"Bearer {self.api_key}"}
        if extra_headers:
            self.headers.update(extra_headers)
        self.temperature = temperature
        self.top_p = top_p

    def complete(self, system: str, user: str, max_tokens: int = 512) -> str:
        payload = {
            "model": self.model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user},
            ],
            "max_tokens": max_tokens,
            "temperature": self.temperature,
            "top_p": self.top_p,
        }
        resp = requests.post(OPENROUTER_URL, headers=self.headers, json=payload, timeout=60)
        resp.raise_for_status()
        data = resp.json()
        try:
            return data["choices"][0]["message"]["content"].strip()
        except Exception as e:
            raise RuntimeError(f"Bad OpenRouter response: {data}") from e
