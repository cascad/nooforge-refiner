use async_trait::async_trait;
use crate::{model::{IngestResult, RagResponse, SearchResult}, server_config::ServerConfig};

pub trait HasConfig {
    fn config(&self) -> &ServerConfig;
}

#[async_trait]
pub trait Pipeline: HasConfig + Send + Sync + 'static {
    async fn ingest_text(&self, text: String, lang: Option<String>, title: Option<String>, explain: Option<bool>) -> anyhow::Result<IngestResult>;
    async fn ingest_url(&self, url: String,  lang: Option<String>, title: Option<String>) -> anyhow::Result<IngestResult>;
    async fn ingest_file(&self, name: String, bytes: Vec<u8>,       lang: Option<String>, title: Option<String>) -> anyhow::Result<IngestResult>;
    async fn search_hybrid(&self, q: String, only_latest: bool, limit: usize) -> anyhow::Result<SearchResult>;
    async fn rag_answer(&self, q: String, limit: usize, stream: bool, model: String, temperature: f32, max_tokens: u32) -> anyhow::Result<RagResponse>;
}

pub mod hybrid;
