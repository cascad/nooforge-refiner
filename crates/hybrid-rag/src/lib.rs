// file: src/lib.rs
//
// Hybrid RAG Rust - библиотека для RAG систем
//
// Основные модули:
// - chunking: Умный чанкинг документов
// - onnx_embedder: Эмбеддинги через ONNX Runtime
// - ingest: Индексация документов в Qdrant
// - query: Поиск и retrieval из Qdrant
// - config: Конфигурация системы

pub mod chunking;
pub mod onnx_embedder;
pub mod ingest;
pub mod query;
pub mod config;
pub mod llm;

// Re-exports для удобства
pub use chunking::{Chunk, ChunkingConfig, chunk_document};
pub use onnx_embedder::ONNXEmbedder;
pub use ingest::{DocumentIndexer, compute_doc_id};
pub use query::{DocumentRetriever, SearchResult};
pub use config::RagConfig;

/// Версия библиотеки
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Простая функция для быстрого старта
/// 
/// # Example
/// 
/// ```no_run
/// use hybrid_rag_rust::quick_start;
/// 
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let (indexer, retriever) = quick_start(
///         "http://localhost:6334",
///         "models/model.onnx",
///         "models/tokenizer.json",
///         "my_collection"
///     ).await?;
///     
///     Ok(())
/// }
/// ```
pub async fn quick_start(
    qdrant_url: &str,
    model_path: &str,
    tokenizer_path: &str,
    collection: &str,
) -> anyhow::Result<(DocumentIndexer, DocumentRetriever)> {
    let config = ChunkingConfig::default();
    
    let indexer = DocumentIndexer::new(
        qdrant_url,
        model_path,
        tokenizer_path,
        collection.to_string(),
        config,
    )
    .await?;
    
    let retriever = DocumentRetriever::new(
        qdrant_url,
        model_path,
        tokenizer_path,
        collection.to_string(),
    )
    .await?;
    
    Ok((indexer, retriever))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_chunking_config_default() {
        let config = ChunkingConfig::default();
        assert!(config.max_tokens > 0);
        assert!(config.overlap_tokens > 0);
    }
}