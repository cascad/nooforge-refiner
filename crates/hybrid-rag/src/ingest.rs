// file: src/ingest.rs
//
// Модуль для индексации документов в Qdrant

use anyhow::Result;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

use qdrant_client::Qdrant;
use qdrant_client::qdrant::{
    point_id::PointIdOptions, 
    Condition, CreateCollectionBuilder, DeletePointsBuilder, Distance, Filter, PointId, PointStruct,
    PointsIdsList, ScrollPointsBuilder, UpsertPointsBuilder, VectorParams, Vectors, VectorsConfig,
    Value,
    vectors_config::Config as VectorsConfigOneOf,
};

use crate::chunking::{chunk_document, Chunk, ChunkingConfig};
use crate::onnx_embedder::ONNXEmbedder;

/// Основной класс для управления индексацией
pub struct DocumentIndexer {
    client: Qdrant,
    embedder: ONNXEmbedder,
    collection: String,
    chunking_config: ChunkingConfig,
}

impl DocumentIndexer {
    /// Создать новый индексатор
    pub async fn new(
        qdrant_url: &str,
        model_path: &str,
        tokenizer_path: &str,
        collection: String,
        chunking_config: ChunkingConfig,
    ) -> Result<Self> {
        let client = Qdrant::from_url(qdrant_url).build()?;
        let embedder = ONNXEmbedder::new(model_path, tokenizer_path)?;

        Ok(Self {
            client,
            embedder,
            collection,
            chunking_config,
        })
    }

    /// Инициализировать коллекцию (создать если не существует)
    pub async fn ensure_collection(&self) -> Result<()> {
        let dim = self.embedder.embed_passage("probe")?.len();

        let collections = self.client.list_collections().await?;
        let exists = collections
            .collections
            .iter()
            .any(|c| c.name == self.collection);

        if !exists {
            self.client
                .create_collection(
                    CreateCollectionBuilder::new(&self.collection).vectors_config(VectorsConfig {
                        config: Some(VectorsConfigOneOf::Params(VectorParams {
                            size: dim as u64,
                            distance: Distance::Cosine.into(),
                            ..Default::default()
                        })),
                    }),
                )
                .await?;
            println!("✨ Created collection: {}", self.collection);
        }
        Ok(())
    }

    /// Индексировать один документ
    pub async fn index_document(
        &self,
        doc_id: &str,
        source_id: &str,
        text: &str,
    ) -> Result<usize> {
        // 1. Чанкинг
        let chunks = chunk_document(doc_id, text, &self.chunking_config);
        if chunks.is_empty() {
            eprintln!("⚠️  WARN: no chunks produced for {}", source_id);
            return Ok(0);
        }

        // 2. Создать points с эмбеддингами
        let (points, keep_ids) = self.create_points(&chunks, doc_id, source_id).await?;

        // 3. Upsert
        self.client
            .upsert_points(UpsertPointsBuilder::new(&self.collection, points))
            .await?;

        // 4. Удалить устаревшие чанки
        self.delete_stale_chunks(doc_id, &keep_ids).await?;

        println!(
            "✅ Indexed: doc_id={}, chunks={}",
            &doc_id[..24.min(doc_id.len())],
            keep_ids.len()
        );

        Ok(keep_ids.len())
    }

    /// Индексировать файл
    pub async fn index_file(&self, path: &std::path::Path, source_id: &str) -> Result<usize> {
        let raw = tokio::fs::read(path).await?;
        let text = String::from_utf8_lossy(&raw).to_string();
        let doc_id = compute_doc_id(&raw);

        self.index_document(&doc_id, source_id, &text).await
    }

    /// Индексировать директорию
    pub async fn index_directory(&self, dir: &str, source_prefix: &str) -> Result<()> {
        let mut rd = tokio::fs::read_dir(dir).await?;
        let mut total_chunks = 0;
        let mut total_docs = 0;

        while let Some(entry) = rd.next_entry().await? {
            if !entry.file_type().await?.is_file() {
                continue;
            }

            let path = entry.path();
            let source_id = format!(
                "{}/{}",
                source_prefix,
                path.file_name().unwrap().to_string_lossy()
            );

            match self.index_file(&path, &source_id).await {
                Ok(chunks) => {
                    total_chunks += chunks;
                    total_docs += 1;
                }
                Err(e) => {
                    eprintln!("❌ Error indexing {:?}: {}", path, e);
                }
            }
        }

        println!(
            "🎉 Indexed {} documents with {} total chunks",
            total_docs, total_chunks
        );
        Ok(())
    }

    /// Удалить документ по doc_id
    pub async fn delete_document(&self, doc_id: &str) -> Result<()> {
        self.delete_stale_chunks(doc_id, &[]).await
    }

    // === Private methods ===

    async fn create_points(
        &self,
        chunks: &[Chunk],
        doc_id: &str,
        source_id: &str,
    ) -> Result<(Vec<PointStruct>, Vec<String>)> {
        let mut points = Vec::with_capacity(chunks.len());
        let mut keep_ids = Vec::with_capacity(chunks.len());

        for chunk in chunks {
            let embedding = self.embedder.embed_passage(&chunk.text)?;
            let numeric_id = chunk_id_to_u64(&chunk.id);

            let mut payload: HashMap<String, Value> = HashMap::new();
            payload.insert("doc_id".into(), Value::from(doc_id.to_string()));
            payload.insert("source_id".into(), Value::from(source_id.to_string()));
            payload.insert("chunk_id".into(), Value::from(chunk.id.clone()));
            payload.insert(
                "span".into(),
                Value::from(vec![
                    Value::from(chunk.start as i64),
                    Value::from(chunk.end as i64),
                ]),
            );
            payload.insert(
                "kinds".into(),
                Value::from(
                    chunk
                        .kind_summary
                        .iter()
                        .map(|k| Value::from(format!("{:?}", k)))
                        .collect::<Vec<_>>(),
                ),
            );
            payload.insert("text_len".into(), Value::from(chunk.text.len() as i64));
            payload.insert("text".into(), Value::from(chunk.text.clone()));

            let point = PointStruct {
                id: Some(PointId {
                    point_id_options: Some(PointIdOptions::Num(numeric_id)),
                }),
                vectors: Some(Vectors::from(embedding)),
                payload: payload.into(),
            };

            keep_ids.push(numeric_id.to_string());
            points.push(point);
        }

        Ok((points, keep_ids))
    }

    async fn delete_stale_chunks(&self, doc_id: &str, keep_ids: &[String]) -> Result<()> {
        let filter = Filter {
            must: vec![Condition::matches("doc_id", doc_id.to_string())],
            ..Default::default()
        };
        let mut existing_ids: Vec<String> = Vec::new();
        let mut next_offset: Option<PointId> = None;

        // Scroll через все чанки этого документа
        loop {
            let mut builder = ScrollPointsBuilder::new(&self.collection)
                .filter(filter.clone())
                .limit(1000)
                .with_payload(false)
                .with_vectors(false);
            
            if let Some(offset) = next_offset.clone() {
                builder = builder.offset(offset);
            }

            let page = self.client.scroll(builder).await?;

            for rec in page.result {
                if let Some(pid) = rec.id {
                    match pid.point_id_options {
                        Some(PointIdOptions::Num(n)) => existing_ids.push(n.to_string()),
                        Some(PointIdOptions::Uuid(u)) => existing_ids.push(u),
                        _ => {}
                    }
                }
            }

            if page.next_page_offset.is_none() {
                break;
            }
            next_offset = page.next_page_offset.clone();
        }

        // Найти ID для удаления
        let keep: HashSet<&String> = keep_ids.iter().collect();
        let to_delete: Vec<PointId> = existing_ids
            .into_iter()
            .filter(|id| !keep.contains(id))
            .filter_map(|id| {
                id.parse::<u64>().ok().map(|num| PointId {
                    point_id_options: Some(PointIdOptions::Num(num)),
                })
            })
            .collect();

        if !to_delete.is_empty() {
            self.client
                .delete_points(
                    DeletePointsBuilder::new(&self.collection)
                        .points(PointsIdsList { ids: to_delete })
                )
                .await?;
        }

        Ok(())
    }
}

// === Helper functions ===

pub fn compute_doc_id(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    format!("doc::{:x}", hasher.finalize())
}

fn chunk_id_to_u64(chunk_id: &str) -> u64 {
    if let Some(hex_part) = chunk_id.strip_prefix("chunk::") {
        u64::from_str_radix(&hex_part[..16.min(hex_part.len())], 16).unwrap_or_else(|_| {
            hash_string_to_u64(chunk_id)
        })
    } else {
        hash_string_to_u64(chunk_id)
    }
}

fn hash_string_to_u64(s: &str) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    let bytes = hasher.finalize();
    u64::from_be_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}