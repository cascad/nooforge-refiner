// file: src/onnx_embedder.rs

use anyhow::{anyhow, Result};
use ndarray::{Array2, ArrayD, CowArray};
use ort::{
    environment::Environment,
    session::{Session, SessionBuilder},
    tensor::OrtOwnedTensor,
    value::Value,
    ExecutionProvider,
    GraphOptimizationLevel,
    LoggingLevel,
};
use std::sync::Arc;
use tokenizers::Tokenizer;

/// Простая обёртка над ONNX-моделью эмбеддингов (совместимо с intfloat/multilingual-e5-base).
pub struct ONNXEmbedder {
    session: Session,
    tokenizer: Tokenizer,
}

impl ONNXEmbedder {
    /// model_path: путь к model.onnx
    /// tokenizer_path: путь к tokenizer.json
    pub fn new(model_path: &str, tokenizer_path: &str) -> Result<Self> {
        // 1) Токенайзер (нужен именно tokenizer.json)
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow!("Failed to load tokenizer from {}: {}", tokenizer_path, e))?;

        // 2) ORT Environment
        let env = Arc::new(
            Environment::builder()
                .with_name("onnx_embedder")
                .with_log_level(LoggingLevel::Warning)
                .build()?,
        );

        // 3) SessionBuilder (ort = 1.16.3 → GraphOptimizationLevel::Level3)
        let mut builder = SessionBuilder::new(&env)?
            .with_optimization_level(GraphOptimizationLevel::Level3)?;

        // GPU / CPU выбор через переменную окружения:
        // ORT_USE=cuda      → CUDA EP (нужна GPU-сборка onnxruntime)
        // ORT_USE=directml  → DirectML EP (Windows без CUDA)
        // иначе             → CPU
        match std::env::var("ORT_USE").unwrap_or_default().to_lowercase().as_str() {
            "cuda" => {
                builder =
                    builder.with_execution_providers([ExecutionProvider::CUDA(Default::default())])?;
            }
            "directml" => {
                builder = builder
                    .with_execution_providers([ExecutionProvider::DirectML(Default::default())])?;
            }
            _ => {
                // CPU подхватится по умолчанию
            }
        }

        let session = builder.with_model_from_file(model_path)?;

        Ok(Self { session, tokenizer })
    }

    /// Базовый эмбеддинг (mean-pooling + L2-нормализация).
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let enc = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| anyhow!("Tokenization failed: {}", e))?;

        // ids/attention_mask как i64
        let ids_i64: Vec<i64> = enc.get_ids().iter().map(|&x| x as i64).collect();
        let seq = ids_i64.len();
        let attn_i64: Vec<i64> = vec![1; seq];

        // Входы как Array2<i64> (форма из usize)
        let ids_a2: Array2<i64> =
            Array2::from_shape_vec((1, seq), ids_i64).map_err(|e| anyhow!("shape ids: {}", e))?;
        let mask_a2: Array2<i64> =
            Array2::from_shape_vec((1, seq), attn_i64).map_err(|e| anyhow!("shape mask: {}", e))?;

        // Преобразуем к динамической размерности и Cow-view
        let ids_dyn: ArrayD<i64> = ids_a2.into_dyn();
        let mask_dyn: ArrayD<i64> = mask_a2.into_dyn();
        let ids_cow: CowArray<'_, i64, _> = CowArray::from(ids_dyn);
        let mask_cow: CowArray<'_, i64, _> = CowArray::from(mask_dyn);

        // Упаковка во Value и запуск
        let ids_val = Value::from_array(self.session.allocator(), &ids_cow)?;
        let mask_val = Value::from_array(self.session.allocator(), &mask_cow)?;
        let outputs = self.session.run(vec![ids_val, mask_val])?;

        // Первый выход: [batch, seq, hidden]
        let embs: OrtOwnedTensor<f32, _> = outputs
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No outputs from ONNX session"))?
            .try_extract()?;

        let view = embs.view();
        let dims = view.shape();
        if dims.len() != 3 {
            return Err(anyhow!("Unexpected output shape: {:?}", dims));
        }
        let (_b, s, h) = (dims[0], dims[1], dims[2]);

        // Mean-pooling
        let mut pooled = vec![0.0f32; h];
        for j in 0..h {
            let mut sum = 0.0f32;
            for i in 0..s {
                sum += view[[0, i, j]];
            }
            pooled[j] = sum / s as f32;
        }

        // L2-нормализация (важно для cosine-поиска)
        let mut norm = 0f32;
        for v in &pooled {
            norm += v * v;
        }
        norm = norm.sqrt();
        if norm > 0.0 {
            for v in &mut pooled {
                *v /= norm;
            }
        }

        Ok(pooled)
    }

    /// E5-режим: эмбеддинг запроса (добавляет префикс `query:`).
    pub fn embed_query(&self, text: &str) -> Result<Vec<f32>> {
        self.embed(&format!("query: {}", text))
    }

    /// E5-режим: эмбеддинг документа/чанка (добавляет префикс `passage:`).
    pub fn embed_passage(&self, text: &str) -> Result<Vec<f32>> {
        self.embed(&format!("passage: {}", text))
    }
}
