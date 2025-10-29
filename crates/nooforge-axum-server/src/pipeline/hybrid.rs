// /src/pipeline/hybrid.rs
// Прямое использование крейта `hybrid-rag` без CLI.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Result};
use async_trait::async_trait;

use super::Pipeline;
use crate::model::{Chunk, IngestResult, RagResponse, SearchResult};
use crate::pipeline::HasConfig;
use crate::server_config::ServerConfig;

use hybrid_rag::chunking::ChunkingConfig;
use hybrid_rag::ingest::{compute_doc_id, DocumentIndexer};
use hybrid_rag::llm::{LlmClient, LlmConfig};
use hybrid_rag::query::{DocumentRetriever, SearchResult as HybridSearchResult};

use chardetng::EncodingDetector;

pub struct HybridPipeline {
    cfg: Arc<ServerConfig>,
    indexer: DocumentIndexer,
    retriever: DocumentRetriever,
}

impl HasConfig for HybridPipeline {
    #[inline]
    fn config(&self) -> &ServerConfig {
        &self.cfg
    }
}

impl HybridPipeline {
    pub async fn new(cfg: Arc<ServerConfig>) -> Result<Self> {
        let qdrant_url = format!(
            "http://{}:{}",
            cfg.hybrid.qdrant_host, cfg.hybrid.qdrant_port
        );

        // Корректно собираем пути
        let model_path = PathBuf::from(&cfg.hybrid.model_dir).join("model.onnx");
        let tokenizer_path = {
            let p = PathBuf::from(&cfg.hybrid.tokenizer_path);
            if p.is_absolute() {
                p
            } else {
                PathBuf::from(&cfg.hybrid.model_dir).join(p)
            }
        };

        // Проверка существования
        if !model_path.exists() {
            anyhow::bail!("Model file not found: {}", model_path.display());
        }
        if !tokenizer_path.exists() {
            anyhow::bail!("Tokenizer file not found: {}", tokenizer_path.display());
        }

        // Преобразуем пути в строки, как ждёт hybrid-rag
        let model_path_str = model_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 in model path"))?;
        let tokenizer_path_str = tokenizer_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 in tokenizer path"))?;

        let chunking = ChunkingConfig {
            max_tokens: cfg.hybrid.max_tokens,
            overlap_tokens: cfg.hybrid.overlap_tokens,
            approx_chars_per_token: 4.0,
            hard_max_bytes: 96 * 1024,
        };

        let indexer = DocumentIndexer::new(
            &qdrant_url,
            model_path_str,
            tokenizer_path_str,
            cfg.hybrid.qdrant_collection.clone(),
            chunking,
        )
        .await?;

        indexer.ensure_collection().await?;

        let retriever = DocumentRetriever::new(
            &qdrant_url,
            model_path_str,
            tokenizer_path_str,
            cfg.hybrid.qdrant_collection.clone(),
        )
        .await?;

        Ok(Self {
            cfg,
            indexer,
            retriever,
        })
    }

    fn map_search(results: Vec<HybridSearchResult>) -> SearchResult {
        let chunks = results
            .into_iter()
            .map(|r| Chunk {
                id: r.chunk_id,
                source: r.source_id,
                title: None,
                kind: if r.kinds.is_empty() {
                    None
                } else {
                    Some(r.kinds.join(","))
                },
                span: Some((r.span.0 as u64, r.span.1 as u64)),
                preview: Some(r.text),
                created_at: None,
            })
            .collect();
        SearchResult { chunks }
    }

    /// Базовая LLM-конфигурация (из self.cfg.llm)
    fn llm_base_config(&self) -> LlmConfig {
        let llm = &self.cfg.llm;
        LlmConfig {
            api_key: llm
                .api_key
                .clone()
                .expect("LLM api_key must be set in ServerConfig"),
            model: llm.model_primary.clone(),
            base_url: llm.base_url.clone(),
            max_tokens: llm.max_tokens as usize,
            temperature: llm.temperature,
        }
    }

    /// Список моделей: preferred (если есть) + fallbacks из self.cfg.llm
    fn model_candidates(&self, preferred: Option<&str>) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(p) = preferred {
            out.push(p.to_string());
        } else {
            out.push(self.cfg.llm.model_primary.clone());
        }
        // добавим фолбэки, уберём дубли
        for m in &self.cfg.llm.model_fallbacks {
            if !out.iter().any(|x| x == m) {
                out.push(m.clone());
            }
        }
        out
    }
}

#[async_trait]
impl Pipeline for HybridPipeline {
    async fn ingest_text(
        &self,
        text: String,
        _lang: Option<String>,
        title: Option<String>,
        _explain: Option<bool>,
    ) -> Result<IngestResult> {
        let doc_id = title.unwrap_or_else(|| compute_doc_id(text.as_bytes()));
        let source_id = format!("{}{}", self.cfg.hybrid.source_prefix, doc_id);

        // индексируем
        self.indexer
            .index_document(&doc_id, &source_id, &text)
            .await?;
        tracing::info!("✅ indexed: {} -> {}", doc_id, source_id);

        // пробуем вытащить чанки сразу после записи
        let mut listed = vec![];

        if let Ok(res) = self.retriever.search_in_document("*", &doc_id, 100).await {
            if !res.is_empty() {
                listed = res;
            }
        }
        if listed.is_empty() {
            // fallback: иногда doc_id != source_id
            if let Ok(res) = self
                .retriever
                .search_in_document("*", &source_id, 100)
                .await
            {
                listed = res;
            }
        }

        tracing::info!("🧩 chunks found = {}", listed.len());
        Ok(IngestResult {
            chunks: listed
                .into_iter()
                .map(|r| Chunk {
                    id: r.chunk_id,
                    source: r.source_id,
                    title: None,
                    kind: if r.kinds.is_empty() {
                        None
                    } else {
                        Some(r.kinds.join(","))
                    },
                    span: Some((r.span.0 as u64, r.span.1 as u64)),
                    preview: Some(r.text),
                    created_at: None,
                })
                .collect(),
            source_id,
        })
    }

    async fn ingest_url(
        &self,
        _url: String,
        _lang: Option<String>,
        _title: Option<String>,
    ) -> Result<IngestResult> {
        bail!("ingest_url is not implemented for the current hybrid-rag API");
    }

    async fn ingest_file(
        &self,
        name: String,
        bytes: Vec<u8>,
        _lang: Option<String>,
        title: Option<String>,
    ) -> Result<IngestResult> {
        // Если файл похож на текст — декодируем и индексируем как документ (UTF-8)
        if is_text_like(&name, &bytes) {
            let text = decode_to_utf8_lossy(&bytes);
            let doc_id = title
                .or_else(|| {
                    std::path::Path::new(&name)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| hybrid_rag::ingest::compute_doc_id(&bytes));
            let source_id = format!("{}{}", self.cfg.hybrid.source_prefix, doc_id);
            self.indexer
                .index_document(&doc_id, &source_id, &text)
                .await?;
            return Ok(IngestResult {
                chunks: vec![],
                source_id,
            });
        }

        // иначе — как было: сохраняем и индексируем папку (pdf/docx и т.п.)
        let dir = tempfile::tempdir()?;
        let path = dir.path().join(&name);
        std::fs::write(&path, &bytes)?;
        let source_id = self.cfg.hybrid.source_prefix.clone();
        self.indexer
            .index_directory(dir.path().to_str().unwrap(), &source_id)
            .await?;
        Ok(IngestResult {
            chunks: vec![],
            source_id,
        })
    }

    async fn search_hybrid(
        &self,
        q: String,
        _only_latest: bool,
        limit: usize,
    ) -> Result<SearchResult> {
        let res = self.retriever.search(&q, limit).await?;
        Ok(Self::map_search(res))
    }

    async fn rag_answer(
        &self,
        q: String,
        limit: usize,
        stream: bool,
        model: String,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<RagResponse> {
        let context = self.retriever.get_context(&q, limit).await?;

        let mut base = self.llm_base_config();
        base.model = model.clone();
        base.temperature = temperature;
        base.max_tokens = max_tokens as usize;

        // 3) кандидаты моделей: preferred (из запроса) + fallbacks (из конфига)
        let mut models = self.model_candidates(Some(&model));
        if models.is_empty() || models[0] != model {
            // обеспечим, что первая попытка — именно запросная модель
            models.insert(0, model.clone());
        }

        // пробуем модели по очереди, для каждой — до 3 ретраев на 429/5xx/timeout
        for (mi, m) in models.iter().enumerate() {
            let mut attempt = 0usize;
            loop {
                attempt += 1;
                let mut cfg = base.clone();
                cfg.model = m.clone();
                println!("config: {:?}", cfg);

                let llm = LlmClient::new(cfg.clone())?;
                let res = if stream {
                    // собираем стрим целиком
                    let mut buf = String::new();
                    let mut first = true;
                    let r = llm
                        .rag_query_stream(&q, &context, |chunk| {
                            // chunk — уже UTF-8
                            if first {
                                first = false;
                            }
                            buf.push_str(&chunk);
                        })
                        .await;
                    r.map(|_| RagResponse {
                        answer: buf,
                        context: context.clone(),
                    })
                } else {
                    llm.rag_query(&q, &context).await.map(|answer| RagResponse {
                        answer,
                        context: context.clone(),
                    })
                };

                match res {
                    Ok(resp) => {
                        tracing::info!(
                            "RAG OK via model='{}' (attempt {} of model idx {})",
                            m,
                            attempt,
                            mi
                        );
                        return Ok(resp);
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        let transient = msg.contains("429")
                        || msg.contains("5")  // 5xx
                        || msg.contains("timeout")
                        || msg.contains("Temporary")
                        || msg.contains("Gateway");
                        let backoff = match attempt {
                            1 => 200,
                            2 => 700,
                            _ => 1500,
                        };
                        tracing::warn!(
                            "RAG model='{}' attempt={} failed: {}{}",
                            m,
                            attempt,
                            msg,
                            if transient {
                                format!(", retry in {}ms", backoff)
                            } else {
                                "".into()
                            }
                        );
                        if transient && attempt < 3 {
                            tokio::time::sleep(std::time::Duration::from_millis(backoff)).await;
                            continue;
                        }
                        // исчерпали попытки — переходим к следующей модели
                        break;
                    }
                }
            }
            tracing::warn!(
                "RAG fallback → next model (idx {} of {})",
                mi + 1,
                models.len()
            );
        }

        anyhow::bail!("All LLM models failed after retries")
    }
}

/// Пытается угадать кодировку и вернуть текст в UTF-8.
fn decode_to_utf8_lossy(bytes: &[u8]) -> String {
    let mut det = EncodingDetector::new();
    det.feed(bytes, true);
    let enc = det.guess(None, true);
    let (cow, _, _) = enc.decode(bytes);
    cow.into_owned()
}

/// Эвристика: определяем, похож ли файл на текст.
fn is_text_like(name: &str, sample: &[u8]) -> bool {
    let ext = std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    // по расширению
    if matches!(
        ext.as_str(),
        "txt" | "md" | "csv" | "json" | "yaml" | "yml" | "html" | "htm" | "rtf" | "log"
    ) {
        return true;
    }

    // по содержимому — если не более 10% байтов «непечатаемые»
    let non_text = sample
        .iter()
        .filter(|&&b| {
            (b < 0x09 || (b > 0x0D && b < 0x20)) && b != 0x1B // esc seq
        })
        .count();
    let ratio = non_text as f32 / sample.len().max(1) as f32;
    ratio < 0.1
}
