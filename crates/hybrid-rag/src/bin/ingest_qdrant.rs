// file: src/bin/ingest_qdrant_grpc.rs

use anyhow::Result;
use clap::Parser;
use hybrid_rag::onnx_embedder::ONNXEmbedder;
use serde_json::json;
use sha1::{Digest, Sha1};

use qdrant_client::qdrant::{points_client::PointsClient, PointStruct, UpsertPoints, Vectors};
use tokio::fs;

#[derive(Parser, Debug)]
#[command(name = "ingest_qdrant_grpc")]
#[command(about = "Залить один текст в Qdrant через gRPC (Points/Upsert)", long_about = None)]
struct Args {
    #[arg(long)]
    input_dir: Option<String>,

    #[arg(long, default_value = None)]
    text: Option<String>,

    #[arg(long, default_value = "demo-source")]
    source_id: String,

    #[arg(long, default_value = "chunks")]
    collection: String,

    #[arg(long, default_value = "models/multilingual-e5-base")]
    model_dir: String,

    #[arg(long, default_value = "models/multilingual-e5-base/tokenizer.json")]
    tokenizer_path: String,

    #[arg(long, default_value = "127.0.0.1")]
    qdrant_host: String,

    #[arg(long, default_value_t = 6334)]
    qdrant_grpc_port: u16,

    #[arg(long, default_value_t = true)]
    wait: bool,
}

fn make_point_ids(source_id: &str, text: &str) -> (String, u64) {
    let mut sha = Sha1::new();
    sha.update(source_id.as_bytes());
    sha.update(b"::");
    sha.update(text.as_bytes());
    let digest = sha.finalize();
    let hex_id = format!("{source_id}::{:x}", digest);
    let mut first8 = [0u8; 8];
    first8.copy_from_slice(&digest[..8]);
    let num_id = u64::from_be_bytes(first8);
    (hex_id, num_id)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // создаём embedder
    let model_path = format!("{}/model.onnx", &args.model_dir);
    let embedder = ONNXEmbedder::new(&model_path, &args.tokenizer_path)?;

    let mut points_batch = Vec::new();

    // либо одиночный текст, либо батч
    if let Some(dir) = &args.input_dir {
        let mut dir_entries = fs::read_dir(dir).await?;

        while let Some(entry) = dir_entries.next_entry().await? {
            let path = entry.path();
            if entry.file_type().await?.is_file() {
                let text = fs::read_to_string(&path).await?;
                let vecf = embedder.embed_passage(&text)?;

                let (point_id_hex, point_id_num) =
                    make_point_ids(&path.display().to_string(), &text);

                let payload_map = json!({
                    "source_id": path.display().to_string(),
                    "len": text.len(),
                    "point_id": point_id_hex,
                })
                .as_object()
                .unwrap()
                .clone();

                points_batch.push(PointStruct::new(
                    point_id_num,
                    Vectors::from(vecf),
                    payload_map,
                ));
            }
        }
    } else if let Some(raw_text) = args.text {
        let text = raw_text.as_str();
        let vecf = embedder.embed_passage(text)?;
        let (point_id_hex, point_id_num) = make_point_ids(&args.source_id, text);
        let payload_map = json!({
            "source_id": args.source_id,
            "len": text.len(),
            "point_id": point_id_hex,
        })
        .as_object()
        .unwrap()
        .clone();
        points_batch.push(PointStruct::new(
            point_id_num,
            Vectors::from(vecf),
            payload_map,
        ));
    }

    let endpoint = format!("http://{}:{}/", args.qdrant_host, args.qdrant_grpc_port);
    let mut points = PointsClient::connect(endpoint).await?;

    let req = UpsertPoints {
        collection_name: args.collection.clone(),
        wait: Some(args.wait),
        points: points_batch,
        ordering: None,
        shard_key_selector: None,
    };

    let _resp = points.upsert(req).await?;

    println!("✅ gRPC upsert ok (batch={})", args.source_id);
    Ok(())
}
