// file: src/bin/search.rs
//
// Бинарник для поиска в Qdrant
//
// Использование:
//   cargo run --bin search -- "How to use Rust?" --limit 5
//   cargo run --bin search -- "машинное обучение" --doc-id doc::abc123
//

use anyhow::Result;
use clap::Parser;

use hybrid_rag::query::DocumentRetriever;

#[derive(Parser, Debug)]
#[command(name = "search")]
#[command(about = "Search documents in Qdrant vector store")]
struct Args {
    /// Search query
    query: String,

    /// Number of results to return
    #[arg(short, long, default_value_t = 5)]
    limit: usize,

    /// Filter by specific document ID
    #[arg(long)]
    doc_id: Option<String>,

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

    /// Output format: text, json
    #[arg(long, default_value = "text")]
    format: String,

    /// Get context string for RAG (combines all results)
    #[arg(long)]
    context: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize retriever
    let qdrant_url = format!("http://{}:{}", args.qdrant_host, args.qdrant_port);
    let model_path = format!("{}/model.onnx", args.model_dir);

    println!("🔍 Searching for: \"{}\"", args.query);

    let retriever = DocumentRetriever::new(
        &qdrant_url,
        &model_path,
        &args.tokenizer_path,
        args.collection.clone(),
    )
    .await?;

    // Search
    let results = if let Some(doc_id) = &args.doc_id {
        retriever
            .search_in_document(&args.query, doc_id, args.limit)
            .await?
    } else {
        retriever.search(&args.query, args.limit).await?
    };

    // Output results
    if args.context {
        let context = retriever.get_context(&args.query, args.limit).await?;
        println!("\n📚 Context for RAG:\n");
        println!("{}", context);
    } else {
        match args.format.as_str() {
            "json" => {
                println!("{}", serde_json::to_string_pretty(&results)?);
            }
            _ => {
                print_results_text(&results);
            }
        }
    }

    Ok(())
}

fn print_results_text(results: &[hybrid_rag::query::SearchResult]) {
    if results.is_empty() {
        println!("\n❌ No results found");
        return;
    }

    println!("\n✅ Found {} results:\n", results.len());

    for (i, result) in results.iter().enumerate() {
        println!("{}. Score: {:.4}", i + 1, result.score);
        println!("   Source: {}", result.source_id);
        println!("   Chunk: {} (span: {}..{})", result.chunk_id, result.span.0, result.span.1);
        if !result.kinds.is_empty() {
            println!("   Kinds: {}", result.kinds.join(", "));
        }
        println!("   Text:");
        
        // Print text with indentation
        for line in result.text.lines() {
            println!("      {}", line);
        }
        println!();
    }
}