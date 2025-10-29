// file: src/bin/search_qdrant.rs

use anyhow::Result;
use clap::Parser;

use hybrid_rag::onnx_embedder::ONNXEmbedder;
use qdrant_client::qdrant::{
    condition::ConditionOneOf, points_client::PointsClient, r#match::MatchValue, Condition,
    FieldCondition, Filter, Match, SearchPoints, WithPayloadSelector, WithVectorsSelector,
};

#[derive(Parser, Debug)]
#[command(name = "search_qdrant")]
#[command(about = "Поиск по Qdrant через gRPC (Points/Search) с E5-эмбеддингом")]
struct Args {
    /// Текст запроса
    #[arg(long)]
    query: String,

    /// Имя коллекции
    #[arg(long, default_value = "chunks")]
    collection: String,

    /// Папка модели (model.onnx + tokenizer.json)
    #[arg(long, default_value = "models/multilingual-e5-base")]
    model_dir: String,

    /// Путь к tokenizer.json
    #[arg(long, default_value = "models/multilingual-e5-base/tokenizer.json")]
    tokenizer_path: String,

    /// Хост Qdrant (gRPC)
    #[arg(long, default_value = "127.0.0.1")]
    qdrant_host: String,

    /// gRPC-порт (обычно 6334)
    #[arg(long, default_value_t = 6334)]
    qdrant_grpc_port: u16,

    /// top-k результатов
    #[arg(long, default_value_t = 5)]
    top_k: u64,

    /// Порог похожести (Cosine), опционально
    #[arg(long)]
    score_threshold: Option<f32>,

    /// Возвращать payload документов
    #[arg(long, default_value_t = true, value_parser = clap::builder::BoolishValueParser::new())]
    with_payload: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // 1) Эмбеддер и вектор запроса (E5: "query: ...")
    let model_path = format!("{}/model.onnx", &args.model_dir);
    let embedder = ONNXEmbedder::new(&model_path, &args.tokenizer_path)?;
    let qvec: Vec<f32> = embedder.embed_query(&args.query)?; // L2-норм есть внутри

    // 2) gRPC-клиент
    let endpoint = format!("http://{}:{}/", args.qdrant_host, args.qdrant_grpc_port);
    let mut points = PointsClient::connect(endpoint).await?;

    // 3) селекторы
    let with_payload = if args.with_payload {
        Some(WithPayloadSelector::from(true)) // вернуть весь payload
    } else {
        None
    };

    let with_vectors = Some(WithVectorsSelector::from(true));

    // хотим искать только по source_id = "samples/lsm-notes.md"
    let field = FieldCondition {
        key: "source_id".to_string(),
        r#match: Some(Match {
            match_value: Some(MatchValue::Keyword("samples/lsm-notes.md".to_string())),
        }),
        ..Default::default()
    };

    let filter = Filter {
        must: vec![Condition {
            condition_one_of: Some(ConditionOneOf::Field(field)),
        }],
        should: vec![],
        must_not: vec![],
        min_should: None,
    };

    // 4) Формируем запрос под 1.15: ВСЕ дополнительные поля явно
    let req = SearchPoints {
        collection_name: args.collection.clone(),
        vector: qvec,      // <- прямой вектор запроса (Vec<f32>)
        vector_name: None, // <- если будет несколько vector fields — укажешь тут Some("...".into())
        limit: args.top_k,
        with_payload, // <- вернуть payload, если нужно
        with_vectors, // <- можно запросить возврат самих векторов, нам не надо
        params: None, // <- SearchParams, если захотим nprobe и т.п.
        score_threshold: args.score_threshold,
        offset: None,
        filter: Some(filter), // <- Filter, если нужен фильтрационный поиск
        read_consistency: None,
        timeout: None,
        shard_key_selector: None, // <- для шардинга, не используем
        sparse_indices: None,     // <- для sparse retrieval, не используем
    };

    // 5) Вызов
    let resp = points.search(req).await?.into_inner();

    println!("✅ hits: {}", resp.result.len());
    for (i, r) in resp.result.iter().enumerate() {
        let id_str = r.id.as_ref().map(|x| format!("{x:?}")).unwrap_or_default();
        println!("#{:<2} score={:.4} id={}", i + 1, r.score, id_str);
        if r.payload.len() > 0 {
            println!("    payload: {}", serde_json::to_string(&r.payload)?);
        }
    }

    Ok(())
}
