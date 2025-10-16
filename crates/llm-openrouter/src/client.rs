// crates/llm-openrouter/src/client.rs

use anyhow::Context;
use llm_traits::{ChatMessage, ChatRequest, ChatResponse, ChatResponseChoice, LlmClient};
use serde_json::json;

pub struct OpenRouterClient {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl OpenRouterClient {
    pub fn new(api_key: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key,
            base_url: "https://openrouter.ai/api/v1".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl LlmClient for OpenRouterClient {
    async fn chat(&self, req: ChatRequest) -> anyhow::Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.base_url);
        let body = json!({
            "model": req.model,
            "messages": req.messages,
            "temperature": req.temperature.unwrap_or(0.1),
            "max_tokens": req.max_tokens,
        });

        let resp = self
            .http
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .context("openrouter request")?;

        let resp_status = resp.status();

        if !resp_status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("openrouter http {}: {}", resp_status, text));
        }

        let v: serde_json::Value = resp.json().await.context("openrouter json")?;

        let mut mapped = Vec::new();
        if let Some(choices) = v.get("choices").and_then(|c| c.as_array()) {
            for (i, ch) in choices.iter().enumerate() {
                let msg = ch.get("message").cloned().unwrap_or_default();
                let role = msg
                    .get("role")
                    .and_then(|r| r.as_str())
                    .unwrap_or("assistant")
                    .to_string();
                let content = msg
                    .get("content")
                    .and_then(|c| c.as_str())
                    .unwrap_or_default()
                    .to_string();

                mapped.push(ChatResponseChoice {
                    index: i,
                    message: ChatMessage { role, content },
                });
            }
        }

        Ok(ChatResponse { choices: mapped })
    }
}
