from llama_index.llms.openai import OpenAI as LIOpenAI
llm = LIOpenAI(
    model="anthropic/claude-sonnet-4.5",
    api_key="sk-or-",
    base_url="https://openrouter.ai/api/v1",
    default_headers={"HTTP-Referer":"http://localhost","X-Title":"nooforge-refiner"},
)
print(llm.complete("Say 'ok'").text)