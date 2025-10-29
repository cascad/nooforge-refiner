// file: src/bin/verify.rs
//
// Проверка коллекции на дубликаты и статистика
//
// Использование:
//   cargo run --bin verify -- --collection chunks
//

use anyhow::Result;
use clap::Parser;
use std::collections::HashMap;

use qdrant_client::qdrant::ScrollPointsBuilder;
use qdrant_client::Qdrant;

#[derive(Parser, Debug)]
#[command(name = "verify")]
#[command(about = "Verify collection and check for duplicates")]
struct Args {
    /// Qdrant collection name
    #[arg(long, default_value = "chunks")]
    collection: String,

    /// Qdrant host
    #[arg(long, env = "QDRANT_HOST", default_value = "127.0.0.1")]
    qdrant_host: String,

    /// Qdrant port
    #[arg(long, env = "QDRANT_PORT", default_value_t = 6334)]
    qdrant_port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let url = format!("http://{}:{}", args.qdrant_host, args.qdrant_port);
    let client = Qdrant::from_url(&url).build()?;

    println!("🔍 Analyzing collection: {}", args.collection);
    println!();

    // Получить инфо о коллекции
    let collection_info = client.collection_info(&args.collection).await?;
    let total_points = collection_info.result.unwrap().points_count.unwrap_or(0);

    println!("📊 Total points: {}", total_points);

    // Собрать статистику по doc_id
    let mut doc_stats: HashMap<String, Vec<String>> = HashMap::new();
    let mut total_chunks = 0;

    let mut next_offset = None;
    loop {
        let mut builder = ScrollPointsBuilder::new(&args.collection)
            .limit(1000)
            .with_payload(true)
            .with_vectors(false);

        if let Some(offset) = next_offset {
            builder = builder.offset(offset);
        }

        let page = client.scroll(builder).await?;

        for point in &page.result {
            total_chunks += 1;

            let payload = &point.payload;
            // Извлечь doc_id
            if let Some(doc_id_value) = payload.get("doc_id") {
                if let Some(doc_id) = extract_string_from_value(doc_id_value) {
                    // Извлечь chunk_id
                    let chunk_id = if let Some(chunk_id_value) = payload.get("chunk_id") {
                        extract_string_from_value(chunk_id_value)
                            .unwrap_or_else(|| "unknown".to_string())
                    } else {
                        "unknown".to_string()
                    };

                    doc_stats
                        .entry(doc_id)
                        .or_insert_with(Vec::new)
                        .push(chunk_id);
                }
            }
        }

        if page.next_page_offset.is_none() {
            break;
        }
        next_offset = page.next_page_offset;
    }

    println!("📚 Total documents: {}", doc_stats.len());
    println!("📄 Total chunks: {}", total_chunks);
    println!();

    // Проверка на дубликаты chunk_id
    println!("🔎 Checking for duplicate chunks...");
    let mut has_duplicates = false;

    for (doc_id, chunk_ids) in &doc_stats {
        let unique_chunks: std::collections::HashSet<_> = chunk_ids.iter().collect();
        if unique_chunks.len() != chunk_ids.len() {
            has_duplicates = true;
            let duplicates = chunk_ids.len() - unique_chunks.len();
            println!(
                "⚠️  Doc {} has {} duplicate chunks!",
                &doc_id[..24.min(doc_id.len())],
                duplicates
            );
        }
    }

    if !has_duplicates {
        println!("✅ No duplicate chunks found!");
    }
    println!();

    // Топ-5 документов по количеству чанков
    println!("📊 Top 5 documents by chunk count:");
    let mut sorted_docs: Vec<_> = doc_stats.iter().collect();
    sorted_docs.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    for (i, (doc_id, chunks)) in sorted_docs.iter().take(5).enumerate() {
        println!(
            "  {}. {} - {} chunks",
            i + 1,
            &doc_id[..24.min(doc_id.len())],
            chunks.len()
        );
    }
    println!();

    // Статистика по source_id
    println!("📁 Documents by source:");
    let mut source_stats: HashMap<String, usize> = HashMap::new();

    let mut next_offset = None;
    loop {
        let mut builder = ScrollPointsBuilder::new(&args.collection)
            .limit(1000)
            .with_payload(true)
            .with_vectors(false);

        if let Some(offset) = next_offset {
            builder = builder.offset(offset);
        }

        let page = client.scroll(builder).await?;

        for point in &page.result {
            let payload = &point.payload;
            if let Some(source_value) = payload.get("source_id") {
                if let Some(source) = extract_string_from_value(source_value) {
                    *source_stats.entry(source).or_insert(0) += 1;
                }
            }
        }

        if page.next_page_offset.is_none() {
            break;
        }
        next_offset = page.next_page_offset;
    }

    for (source, count) in source_stats.iter() {
        println!("  {} - {} chunks", source, count);
    }

    println!();
    println!("✨ Verification complete!");

    Ok(())
}

fn extract_string_from_value(value: &qdrant_client::qdrant::Value) -> Option<String> {
    match value.kind.as_ref()? {
        qdrant_client::qdrant::value::Kind::StringValue(s) => Some(s.clone()),
        _ => None,
    }
}
