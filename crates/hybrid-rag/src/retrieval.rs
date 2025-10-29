use anyhow::Result;
use qdrant_client::qdrant::{
    QdrantClient, 
    SearchPoints, 
    PointStruct,
    vectors::Vector,
    Value, Filter, Condition,
};
use crate::{Document, SearchResult};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RetrievalStrategy {
    Dense,      // Векторный поиск
    Sparse,     // Ключевые слова (BM25-like)
    Hybrid,     // Комбинированный
}

pub struct HybridRetriever {
    qdrant: QdrantClient,
    collection_name: String,
}

impl HybridRetriever {
    pub fn new(qdrant: QdrantClient, collection_name: String) -> Self {
        Self {
            qdrant,
            collection_name,
        }
    }

    pub async fn hybrid_search(
        &self,
        query: &str,
        query_embedding: &[f32],
    ) -> Result<Vec<SearchResult>> {
        let mut all_results = Vec::new();

        // 1. Плотный поиск (векторный)
        let dense_results = self.dense_search(query_embedding, 10).await?;
        all_results.extend(dense_results);

        // 2. Разреженный поиск (ключевые слова)
        let sparse_results = self.sparse_search(query, 10).await?;
        all_results.extend(sparse_results);

        // Дедупликация по ID документа
        let mut seen_ids = std::collections::HashSet::new();
        let mut unique_results = Vec::new();

        for result in all_results {
            if seen_ids.insert(result.document.id.clone()) {
                unique_results.push(result);
            }
        }

        Ok(unique_results)
    }

    async fn dense_search(
        &self,
        embedding: &[f32],
        limit: u64,
    ) -> Result<Vec<SearchResult>> {
        let search_points = SearchPoints {
            collection_name: self.collection_name.clone(),
            vector: embedding.to_vec(),
            limit,
            with_payload: Some(true.into()),
            ..Default::default()
        };

        let response = self.qdrant.search(&search_points).await?;
        
        let results = response.result.into_iter().map(|point| {
            SearchResult {
                document: Document {
                    id: point.id.unwrap().into(),
                    text: point.payload.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    metadata: serde_json::to_value(point.payload).unwrap_or_default(),
                    embedding: None,
                },
                score: point.score,
                strategy: RetrievalStrategy::Dense,
            }
        }).collect();

        Ok(results)
    }

    async fn sparse_search(&self, query: &str, limit: u64) -> Result<Vec<SearchResult>> {
        // Упрощенная реализация BM25-like поиска
        // В реальности можно использовать отдельную коллекцию с BM25 эмбеддингами
        
        let keywords = self.extract_keywords(query);
        let filter = self.build_keyword_filter(&keywords);
        
        let search_points = SearchPoints {
            collection_name: self.collection_name.clone(),
            vector: vec![0.0; 384], // Заглушка
            limit,
            filter: Some(filter),
            with_payload: Some(true.into()),
            ..Default::default()
        };

        let response = self.qdrant.search(&search_points).await?;
        
        let results = response.result.into_iter().map(|point| {
            SearchResult {
                document: Document {
                    id: point.id.unwrap().into(),
                    text: point.payload.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    metadata: serde_json::to_value(point.payload).unwrap_or_default(),
                    embedding: None,
                },
                score: point.score,
                strategy: RetrievalStrategy::Sparse,
            }
        }).collect();

        Ok(results)
    }

    fn extract_keywords(&self, query: &str) -> Vec<String> {
        query.split_whitespace()
            .map(|word| word.to_lowercase())
            .collect()
    }

    fn build_keyword_filter(&self, keywords: &[String]) -> Filter {
        let conditions: Vec<Condition> = keywords.iter()
            .map(|keyword| {
                Condition::matches("text", keyword.clone())
            })
            .collect();

        Filter {
            should: conditions,
            ..Default::default()
        }
    }
}