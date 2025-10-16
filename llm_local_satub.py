from .llm_iface import LLM

class LocalLLMStub(LLM):
    """
    Заглушка: имитирует поведение, возвращая усечённый вход.
    Замените реализацией под свой Ollama/vLLM/LM Studio и т.д.
    """
    def complete(self, system: str, user: str, max_tokens: int = 512) -> str:
        text = user.strip().replace("\n\n", "\n")
        return text[: max_tokens * 4]
