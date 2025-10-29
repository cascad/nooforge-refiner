use anyhow::Result;
use async_trait::async_trait;
use candle_core::{Device, Tensor, D};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use tokenizers::Tokenizer;
use reqwest::Client;
use serde_json::json;

#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}

// Локальный эмбеддер на CPU/GPU
pub struct LocalEmbedder {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl LocalEmbedder {
    pub fn new() -> Result<Self> {
        let device = Device::Cpu;
        
        // Загрузка токенизатора
        let tokenizer = Tokenizer::from_pretrained("sentence-transformers/all-MiniLM-L6-v2", None)?;
        
        // Загрузка конфигурации и модели
        let config = Config::tiny();
        let vb = VarBuilder::from_pretrained("sentence-transformers/all-MiniLM-L6-v2", &device)?;
        let model = BertModel::load(vb, &config)?;
        
        Ok(Self {
            model,
            tokenizer,
            device,
        })
    }
}

#[async_trait]
impl Embedder for LocalEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Токенизация
        let encoding = self.tokenizer.encode(text, true)?;
        let tokens = encoding.get_ids();
        
        // Создание тензоров
        let tokens_tensor = Tensor::new(tokens, &self.device)?.unsqueeze(0)?;
        let token_type_ids = Tensor::zeros((1, tokens.len() as u64), candle_core::DType::U32, &self.device)?;
        
        // Forward pass
        let output = self.model.forward(&tokens_tensor, &token_type_ids)?;
        
        // Усреднение эмбеддингов (mean pooling)
        let embedding = output.mean(1)?.squeeze(0)?;
        let embedding_vec: Vec<f32> = embedding.to_vec1()?;
        
        Ok(embedding_vec)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::new();
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }
}

// Удаленный эмбеддер через API
pub struct RemoteEmbedder {
    client: Client,
    api_url: String,
}

impl RemoteEmbedder {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: Client::new(),
            api_url: "http://localhost:8080/embed".to_string(),
        })
    }
}

#[async_trait]
impl Embedder for RemoteEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let response = self.client
            .post(&self.api_url)
            .json(&json!({ "text": text }))
            .send()
            .await?;
        
        let embedding: Vec<f32> = response.json().await?;
        Ok(embedding)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let response = self.client
            .post(&self.api_url)
            .json(&json!({ "texts": texts }))
            .send()
            .await?;
        
        let embeddings: Vec<Vec<f32>> = response.json().await?;
        Ok(embeddings)
    }
}