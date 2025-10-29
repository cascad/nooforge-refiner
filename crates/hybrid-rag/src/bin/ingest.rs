// file: src/bin/ingest.rs
//
// Упрощённый бинарник для индексации документов в Qdrant
//
// Использование:
//   cargo run --bin ingest -- --input-dir ./docs --collection my_docs
//   cargo run --bin ingest -- --text "Hello world" --doc-id test
//   cargo run --bin ingest -- --config config.toml --input-dir ./docs
//

use anyhow::{bail, Result};
use clap::Parser;

use hybrid_rag::chunking::ChunkingConfig;
use hybrid_rag::ingest::DocumentIndexer;

#[derive(Parser, Debug)]
#[command(name = "ingest")]
#[command(about = "Index documents into Qdrant vector store")]
struct Args {
    /// Input directory with text files
    #[arg(long)]
    input_dir: Option<String>,

    /// Single text to index
    #[arg(long)]
    text: Option<String>,

    /// Document ID for single text (auto-generated if not provided)
    #[arg(long)]
    doc_id: Option<String>,

    /// Source ID prefix
    #[arg(long, default_value = "file://")]
    source_id: String,

    /// Qdrant collection name
    #[arg(long, default_value = "chunks")]
    collection: String,

    /// Model directory
    #[arg(long, default_value = "models/multilingual-e5-base")]
    model_dir: String,

    /// Tokenizer path
    #[arg(long, default_value = "models/multilingual-e5-base/tokenizer.json")]
    tokenizer_path: String,

    /// Qdrant host
    #[arg(long, env = "QDRANT_HOST", default_value = "127.0.0.1")]
    qdrant_host: String,

    /// Qdrant port
    #[arg(long, env = "QDRANT_PORT", default_value_t = 6334)]
    qdrant_port: u16,

    /// Max tokens per chunk
    #[arg(long, default_value_t = 350)]
    max_tokens: usize,

    /// Overlap tokens between chunks
    #[arg(long, default_value_t = 60)]
    overlap_tokens: usize,

    /// Config file path (TOML)
    #[arg(long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Chunking config
    let chunking_config = ChunkingConfig {
        max_tokens: args.max_tokens,
        overlap_tokens: args.overlap_tokens,
        approx_chars_per_token: 4.0,
        hard_max_bytes: 96 * 1024,
    };

    // Initialize indexer
    let qdrant_url = format!("http://{}:{}", args.qdrant_host, args.qdrant_port);
    let model_path = format!("{}/model.onnx", args.model_dir);

    println!("🚀 Initializing indexer...");
    let indexer = DocumentIndexer::new(
        &qdrant_url,
        &model_path,
        &args.tokenizer_path,
        args.collection.clone(),
        chunking_config,
    )
    .await?;

    // Ensure collection exists
    indexer.ensure_collection().await?;

    // Index documents
    if let Some(dir) = args.input_dir {
        println!("📂 Indexing directory: {}", dir);
        indexer.index_directory(&dir, &args.source_id).await?;
    } else if let Some(text) = args.text {
        println!("📝 Indexing single document...");
        let doc_id = args
            .doc_id
            .unwrap_or_else(|| hybrid_rag::ingest::compute_doc_id(text.as_bytes()));
        let source_id = format!("{}{}", args.source_id, doc_id);
        indexer.index_document(&doc_id, &source_id, &text).await?;
    } else {
        bail!("Specify --input-dir OR --text");
    }

    println!("✨ Done!");
    Ok(())
}