// file: src/query.rs
//
// Модуль для поиска документов в Qdrant

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use qdrant_client::qdrant::{Condition, Filter, SearchPointsBuilder};
use qdrant_client::Qdrant;

use crate::onnx_embedder::ONNXEmbedder;

/// Результат поиска
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub score: f32,
    pub doc_id: String,
    pub source_id: String,
    pub chunk_id: String,
    pub text: String,
    pub span: (usize, usize),
    pub kinds: Vec<String>,
}

/// Класс для поиска документов
pub struct DocumentRetriever {
    client: Qdrant,
    embedder: ONNXEmbedder,
    collection: String,
}

impl DocumentRetriever {
    /// Создать новый retriever
    pub async fn new(
        qdrant_url: &str,
        model_path: &str,
        tokenizer_path: &str,
        collection: String,
    ) -> Result<Self> {
        let client = Qdrant::from_url(qdrant_url).build()?;
        let embedder = ONNXEmbedder::new(model_path, tokenizer_path)?;

        Ok(Self {
            client,
            embedder,
            collection,
        })
    }

    /// Поиск похожих документов
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let query_vector = self.embedder.embed_query(query)?;

        let search_result = self
            .client
            .search_points(
                SearchPointsBuilder::new(&self.collection, query_vector, limit as u64)
                    .with_payload(true),
            )
            .await?;

        let mut results = Vec::new();
        for point in search_result.result {
            if let Some(result) = self.parse_search_result(point) {
                results.push(result);
            }
        }

        Ok(results)
    }

    /// Поиск с фильтром по doc_id
    pub async fn search_in_document(
        &self,
        query: &str,
        doc_id: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let query_vector = self.embedder.embed_query(query)?;

        let filter = Filter {
            must: vec![Condition::matches("doc_id", doc_id.to_string())],
            ..Default::default()
        };

        let search_result = self
            .client
            .search_points(
                SearchPointsBuilder::new(&self.collection, query_vector, limit as u64)
                    .filter(filter)
                    .with_payload(true),
            )
            .await?;

        let mut results = Vec::new();
        for point in search_result.result {
            if let Some(result) = self.parse_search_result(point) {
                results.push(result);
            }
        }

        Ok(results)
    }

    /// Поиск с произвольным фильтром
    pub async fn search_with_filter(
        &self,
        query: &str,
        filter: Filter,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let query_vector = self.embedder.embed_query(query)?;

        let search_result = self
            .client
            .search_points(
                SearchPointsBuilder::new(&self.collection, query_vector, limit as u64)
                    .filter(filter)
                    .with_payload(true),
            )
            .await?;

        let mut results = Vec::new();
        for point in search_result.result {
            if let Some(result) = self.parse_search_result(point) {
                results.push(result);
            }
        }

        Ok(results)
    }

    /// Получить контекст для RAG (объединённый текст из топ результатов)
    pub async fn get_context(&self, query: &str, limit: usize) -> Result<String> {
        let results = self.search(query, limit).await?;

        let context = results
            .iter()
            .map(|r| format!("Source: {}\n{}\n", r.source_id, r.text))
            .collect::<Vec<_>>()
            .join("\n---\n\n");

        Ok(context)
    }

    /// Гибридный поиск (семантический + keyword matching)
    /// Использует RRF (Reciprocal Rank Fusion) для объединения результатов
    pub async fn hybrid_search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        // 1. Семантический поиск
        let semantic_results = self.search(query, limit * 2).await?;

        // 2. Простой keyword boost через rescoring
        // (для полноценного BM25 нужен отдельный sparse embedder)
        let boosted = self.boost_keyword_matches(semantic_results, query);

        // Взять топ результаты
        Ok(boosted.into_iter().take(limit).collect())
    }

    /// Буст результатов с keyword совпадениями
    fn boost_keyword_matches(
        &self,
        mut results: Vec<SearchResult>,
        query: &str,
    ) -> Vec<SearchResult> {
        let keywords: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        // Буст для результатов с keyword совпадениями
        for result in &mut results {
            let text_lower = result.text.to_lowercase();
            let mut keyword_count = 0;

            for keyword in &keywords {
                if text_lower.contains(keyword) {
                    keyword_count += text_lower.matches(keyword.as_str()).count();
                }
            }

            // Boost score based on keyword matches
            if keyword_count > 0 {
                result.score += (keyword_count as f32) * 0.1;
            }
        }

        // Re-sort by new scores
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }

    /// Получить контекст для RAG из гибридного поиска
    pub async fn get_hybrid_context(&self, query: &str, limit: usize) -> Result<String> {
        let results = self.hybrid_search(query, limit).await?;

        let context = results
            .iter()
            .map(|r| format!("Source: {}\n{}\n", r.source_id, r.text))
            .collect::<Vec<_>>()
            .join("\n---\n\n");

        Ok(context)
    }

    // === Private methods ===

    fn parse_search_result(
        &self,
        point: qdrant_client::qdrant::ScoredPoint,
    ) -> Option<SearchResult> {
        let id = match point.id?.point_id_options? {
            qdrant_client::qdrant::point_id::PointIdOptions::Num(n) => n.to_string(),
            qdrant_client::qdrant::point_id::PointIdOptions::Uuid(u) => u,
        };

        let payload = point.payload;

        let doc_id = extract_string(&payload, "doc_id")?;
        let source_id = extract_string(&payload, "source_id")?;
        let chunk_id = extract_string(&payload, "chunk_id")?;
        let text = extract_string(&payload, "text")?;

        let span = extract_span(&payload)?;
        let kinds = extract_string_array(&payload, "kinds");

        Some(SearchResult {
            id,
            score: point.score,
            doc_id,
            source_id,
            chunk_id,
            text,
            span,
            kinds,
        })
    }
}

// === Helper functions ===

fn extract_string(
    payload: &HashMap<String, qdrant_client::qdrant::Value>,
    key: &str,
) -> Option<String> {
    payload.get(key).and_then(|v| match v.kind.as_ref()? {
        qdrant_client::qdrant::value::Kind::StringValue(s) => Some(s.clone()),
        _ => None,
    })
}

fn extract_span(payload: &HashMap<String, qdrant_client::qdrant::Value>) -> Option<(usize, usize)> {
    let span_value = payload.get("span")?;

    match span_value.kind.as_ref()? {
        qdrant_client::qdrant::value::Kind::ListValue(list) => {
            if list.values.len() >= 2 {
                let start = match list.values[0].kind.as_ref()? {
                    qdrant_client::qdrant::value::Kind::IntegerValue(n) => *n as usize,
                    _ => return None,
                };
                let end = match list.values[1].kind.as_ref()? {
                    qdrant_client::qdrant::value::Kind::IntegerValue(n) => *n as usize,
                    _ => return None,
                };
                Some((start, end))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn extract_string_array(
    payload: &HashMap<String, qdrant_client::qdrant::Value>,
    key: &str,
) -> Vec<String> {
    payload
        .get(key)
        .and_then(|v| match v.kind.as_ref() {
            Some(qdrant_client::qdrant::value::Kind::ListValue(list)) => Some(
                list.values
                    .iter()
                    .filter_map(|item| match item.kind.as_ref() {
                        Some(qdrant_client::qdrant::value::Kind::StringValue(s)) => Some(s.clone()),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}
