use anyhow::Result;
use tokio::fs;

pub async fn simple_ingest() -> Result<()> {
    println!("🚀 Simple document ingestion started...");
    
    // 1. Читаем все текстовые файлы из папки data
    let data_dir = "data";
    if !std::path::Path::new(data_dir).exists() {
        println!("📁 Creating data directory...");
        tokio::fs::create_dir_all(data_dir).await?;
        println!("✅ Please put your .txt files in the 'data' folder and run again");
        return Ok(());
    }
    
    // 2. Инициализация (упрощенная)
    let embedder = ONNXEmbedder::new(
        "models/all-MiniLM-L6-v2.onnx",
        "models/tokenizer.json"
    )?;
    
    let qdrant = qdrant_client::QdrantClient::connect("http://localhost:6334").await?;
    let ingester = DocumentIngester::new(embedder, qdrant, "documents".to_string());
    
    // 3. Загрузка документов
    println!("📂 Looking for documents in 'data' folder...");
    let results = ingester.ingest_directory(data_dir).await?;
    
    if results.is_empty() {
        println!("❌ No documents found in 'data' folder");
        println!("💡 Add some .txt files and run again");
    } else {
        println!("✅ Successfully ingested {} documents", results.len());
    }
    
    Ok(())
}