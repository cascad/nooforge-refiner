// file: src/bin/rag.rs
//
// Полноценная RAG система с гибридным поиском и LLM
//
// Использование:
//   export OPENROUTER_API_KEY=your_key_here
//   cargo run --bin rag -- "Как работает Rust ownership?"
//   cargo run --bin rag -- "machine learning" --hybrid --stream
//

use anyhow::Result;
use clap::Parser;
use std::io::{self, Write};

use hybrid_rag::query::DocumentRetriever;
use hybrid_rag::llm::{LlmClient, LlmConfig};

#[derive(Parser, Debug)]
#[command(name = "rag")]
#[command(about = "Ask questions using RAG (Retrieval-Augmented Generation)")]
struct Args {
    /// Your question
    query: String,

    /// Number of context chunks to retrieve
    #[arg(short = 'n', long, default_value_t = 5)]
    context_limit: usize,

    /// Use hybrid search (semantic + keyword)
    #[arg(long)]
    hybrid: bool,

    /// Stream the response
    #[arg(long)]
    stream: bool,

    /// Show retrieved context before answer
    #[arg(long)]
    show_context: bool,

    /// LLM model to use
    #[arg(long, default_value = "anthropic/claude-sonnet-4.5")]
    model: String,

    /// LLM temperature (0.0 - 2.0)
    #[arg(long, default_value_t = 0.7)]
    temperature: f32,

    /// Max tokens for LLM response
    #[arg(long, default_value_t = 4096)]
    max_tokens: usize,

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

    /// Skip LLM, only show retrieved context
    #[arg(long)]
    context_only: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Try to load .env file if it exists (optional, doesn't fail if missing)
    let _ = dotenvy::dotenv();
    
    let args = Args::parse();

    println!("🤖 RAG System");
    println!("📝 Question: {}", args.query);
    println!();

    // 1. Initialize retriever
    let qdrant_url = format!("http://{}:{}", args.qdrant_host, args.qdrant_port);
    let model_path = format!("{}/model.onnx", args.model_dir);

    print!("🔍 Searching knowledge base");
    if args.hybrid {
        print!(" (hybrid search)");
    }
    println!("...");

    let retriever = DocumentRetriever::new(
        &qdrant_url,
        &model_path,
        &args.tokenizer_path,
        args.collection.clone(),
    )
    .await?;

    // 2. Retrieve context
    let context = if args.hybrid {
        retriever.get_hybrid_context(&args.query, args.context_limit).await?
    } else {
        retriever.get_context(&args.query, args.context_limit).await?
    };

    if context.trim().is_empty() {
        println!("❌ No relevant information found in knowledge base");
        return Ok(());
    }

    println!("✅ Found {} relevant chunks\n", args.context_limit);

    // Show context if requested
    if args.show_context {
        println!("📚 Retrieved Context:");
        println!("{}", "=".repeat(80));
        println!("{}", context);
        println!("{}", "=".repeat(80));
        println!();
    }

    // If context-only mode, exit here
    if args.context_only {
        return Ok(());
    }

    // 3. Initialize LLM client
    let mut llm_config = LlmConfig::default();
    llm_config.model = args.model.clone();
    llm_config.temperature = args.temperature;
    llm_config.max_tokens = args.max_tokens;
    
    // Get API key from environment
    match std::env::var("OPENROUTER_API_KEY") {
        Ok(key) => {
            if key.is_empty() {
                eprintln!("❌ Error: OPENROUTER_API_KEY is empty!");
                eprintln!("   Get your key at: https://openrouter.ai/keys");
                anyhow::bail!("API key is empty");
            }
            // Show first 10 and last 4 chars for debugging
            let masked = if key.len() > 14 {
                format!("{}...{}", &key[..10], &key[key.len()-4..])
            } else {
                format!("{}...", &key[..key.len().min(10)])
            };
            println!("🔑 API Key loaded: {}", masked);
            llm_config.api_key = key;
        }
        Err(_) => {
            eprintln!("❌ Error: OPENROUTER_API_KEY environment variable not set!");
            eprintln!("   Please set it using:");
            eprintln!("   Windows PowerShell: $env:OPENROUTER_API_KEY=\"sk-or-v1-...\"");
            eprintln!("   Linux/Mac: export OPENROUTER_API_KEY=\"sk-or-v1-...\"");
            eprintln!();
            eprintln!("   Get your key at: https://openrouter.ai/keys");
            anyhow::bail!("OPENROUTER_API_KEY not set");
        }
    }

    let llm = LlmClient::new(llm_config)?;

    // 4. Generate response
    println!("💭 Generating answer with {}...\n", args.model);
    println!("{}", "─".repeat(80));

    if args.stream {
        // Streaming response
        llm.rag_query_stream(&args.query, &context, |chunk| {
            print!("{}", chunk);
            io::stdout().flush().unwrap();
        })
        .await?;
        println!();
    } else {
        // Non-streaming response
        let answer = llm.rag_query(&args.query, &context).await?;
        println!("{}", answer);
    }

    println!("{}", "─".repeat(80));
    println!();
    println!("✨ Done!");

    Ok(())
}