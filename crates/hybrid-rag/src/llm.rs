// file: src/llm.rs
//
// Модуль для работы с LLM через OpenRouter API

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use futures_util::StreamExt;

/// Конфигурация LLM
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub max_tokens: usize,
    pub temperature: f32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "anthropic/claude-sonnet-4.5".to_string(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
        }
    }
}

/// Клиент для работы с LLM
pub struct LlmClient {
    client: Client,
    config: LlmConfig,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: usize,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: Delta,
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Delta {
    #[allow(dead_code)]
    role: Option<String>,
    content: Option<String>,
}

impl LlmClient {
    /// Создать новый LLM клиент
    pub fn new(config: LlmConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self { client, config })
    }

    /// Отправить запрос к LLM и получить ответ
    pub async fn chat(&self, messages: Vec<Message>) -> Result<String> {
        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            stream: None,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.config.base_url))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to LLM")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("LLM API error {}: {}", status, body);
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .context("Failed to parse LLM response")?;

        chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow::anyhow!("No response from LLM"))
    }

    /// Отправить запрос к LLM со streaming ответом
    pub async fn chat_stream<F>(&self, messages: Vec<Message>, mut callback: F) -> Result<String>
    where
        F: FnMut(&str),
    {
        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            stream: Some(true),
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.config.base_url))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to LLM")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("LLM API error {}: {}", status, body);
        }

        let mut stream = response.bytes_stream();
        let mut full_response = String::new();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.context("Failed to read stream chunk")?;
            let chunk_str = String::from_utf8_lossy(&chunk);
            
            buffer.push_str(&chunk_str);

            // Process complete lines
            loop {
                if let Some(line_end) = buffer.find('\n') {
                    let line = buffer[..line_end].trim().to_string();
                    buffer.drain(..line_end + 1);

                    if line.starts_with("data: ") {
                        let data = &line[6..];
                        
                        if data == "[DONE]" {
                            break;
                        }

                        if let Ok(chunk_data) = serde_json::from_str::<StreamChunk>(data) {
                            if let Some(choice) = chunk_data.choices.first() {
                                if let Some(content) = &choice.delta.content {
                                    full_response.push_str(content);
                                    callback(content);
                                }
                            }
                        }
                    }
                } else {
                    break;
                }
            }
        }

        Ok(full_response)
    }

    /// Простой helper для RAG запроса
    pub async fn rag_query(&self, query: &str, context: &str) -> Result<String> {
        let system_message = Message {
            role: "system".to_string(),
            content: "Ты полезный ассистент. Отвечай на вопросы используя предоставленный контекст. Если информации в контексте недостаточно, так и скажи.".to_string(),
        };

        let user_message = Message {
            role: "user".to_string(),
            content: format!(
                "Контекст из базы знаний:\n\n{}\n\n---\n\nВопрос: {}",
                context, query
            ),
        };

        self.chat(vec![system_message, user_message]).await
    }

    /// RAG запрос со streaming
    pub async fn rag_query_stream<F>(
        &self,
        query: &str,
        context: &str,
        callback: F,
    ) -> Result<String>
    where
        F: FnMut(&str),
    {
        let system_message = Message {
            role: "system".to_string(),
            content: "Ты полезный ассистент. Отвечай на вопросы используя предоставленный контекст. Если информации в контексте недостаточно, так и скажи. Отвечай на том же языке, на котором задан вопрос.".to_string(),
        };

        let user_message = Message {
            role: "user".to_string(),
            content: format!(
                "Контекст из базы знаний:\n\n{}\n\n---\n\nВопрос: {}",
                context, query
            ),
        };

        self.chat_stream(vec![system_message, user_message], callback)
            .await
    }
}