# Под рукой несколько адекватных вариантов на OpenRouter (проверьте доступность в вашем тарифе):
# - qwen/qwen-2.5-72b-instruct            — сильный zero-shot RU/EN, хорошо суммирует/рефайнит
# - qwen/qwen-2.5-7b-instruct             — дёшево/быстро, на пробу
# - meta-llama/llama-3.1-70b-instruct     — сильный general-purpose, хорошо держит структуру
# - mistralai/mistral-large-latest        — аккуратные деловые ответы (если доступен)
# - deepseek/deepseek-chat                — дешёвый, но неплохой для рефайна/перефраза
#
# Если нужен vision (не сейчас, но вдруг): qwen/qwen-2.5-vl-72b-instruct
# Для кода: deepseek/deepseek-coder — только если захотите рефайнить блоки кода отдельно

DEFAULT_OPENROUTER_MODEL = "qwen/qwen-2.5-72b-instruct"
