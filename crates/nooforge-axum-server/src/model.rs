use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: String,
    pub source: String,
    pub title: Option<String>,
    pub kind: Option<String>,
    pub span: Option<(u64, u64)>,
    pub preview: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResult {
    pub chunks: Vec<Chunk>,
    pub source_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunks: Vec<Chunk>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(default)]
    pub only_latest: bool,
    #[serde(default = "default_limit")]
    pub limit: usize,
}
fn default_limit() -> usize { 10 }

#[derive(Debug, Clone, Deserialize)]
pub struct RagRequest {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub stream: bool,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}
// todo это не должно настраиваться тут
fn default_model() -> String { "anthropic/claude-sonnet-4.5".into() }
fn default_temperature() -> f32 { 0.2 }
fn default_max_tokens() -> u32 { 512 }

#[derive(Debug, Clone, Serialize)]
pub struct RagResponse {
    pub answer: String,
    pub context: String,
}
