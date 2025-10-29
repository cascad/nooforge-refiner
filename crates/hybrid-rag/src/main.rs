// file: src/main.rs
mod onnx_embedder;

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "embed")]
#[command(about = "ONNX embedding sanity run (multilingual-e5-base)", long_about = None)]
struct Args {
    /// Папка модели (должны лежать model.onnx + tokenizer_config.json + sentencepiece.bpe.model + special_tokens_map.json + config.json)
    #[arg(long, default_value = "models/multilingual-e5-base")]
    model_dir: String,

    /// Явный путь к tokenizer (обычно tokenizer_config.json в той же папке)
    #[arg(long, default_value = "models/multilingual-e5-base/tokenizer_config.json")]
    tokenizer_path: String,

    /// Текст для проверки
    #[arg(long, default_value = "Привет, мир! Это проверка русских эмбеддингов.")]
    text: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let model_path = format!("{}/model.onnx", args.model_dir);

    let embedder = onnx_embedder::ONNXEmbedder::new(&model_path, &args.tokenizer_path)?;
    let emb = embedder.embed(&args.text)?;
    println!("✅ Вектор готов. Длина: {}", emb.len());
    println!("Первые 8 значений: {:?}", &emb[..emb.len().min(8)]);
    Ok(())
}
