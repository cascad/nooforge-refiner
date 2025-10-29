// file: src/config.rs
//
// Конфигурация для RAG системы

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    pub chunking: ChunkingConfig,
    pub embedder: EmbedderConfig,
    pub qdrant: QdrantConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    pub max_tokens: usize,
    pub overlap_tokens: usize,
    pub approx_chars_per_token: f32,
    pub hard_max_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedderConfig {
    pub model_path: String,
    pub tokenizer_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdrantConfig {
    pub host: String,
    pub port: u16,
    pub collection: String,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            max_tokens: 350,
            overlap_tokens: 60,
            approx_chars_per_token: 4.0,
            hard_max_bytes: 96 * 1024,
        }
    }
}

impl Default for EmbedderConfig {
    fn default() -> Self {
        Self {
            model_path: "models/multilingual-e5-base/model.onnx".to_string(),
            tokenizer_path: "models/multilingual-e5-base/tokenizer.json".to_string(),
        }
    }
}

impl Default for QdrantConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 6334,
            collection: "chunks".to_string(),
        }
    }
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            chunking: ChunkingConfig::default(),
            embedder: EmbedderConfig::default(),
            qdrant: QdrantConfig::default(),
        }
    }
}

impl RagConfig {
    /// Загрузить конфигурацию из TOML файла
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: RagConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Сохранить конфигурацию в TOML файл
    pub fn save_to_file(&self, path: &str) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}