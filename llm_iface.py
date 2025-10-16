from typing import Protocol

class LLM(Protocol):
    def complete(self, system: str, user: str, max_tokens: int = 512) -> str: ...
