// /src/main.rs
mod api;
mod model;
mod pipeline;
mod server_config;

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::{
    cors::{Any, CorsLayer},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

use std::{net::SocketAddr, sync::Arc};

use crate::pipeline::hybrid::HybridPipeline;
use crate::{
    api::{health, ingest_file, ingest_text, ingest_text_raw, ingest_url, rag, search, AppState},
    server_config::ServerConfig,
};
use axum::http::header::{HeaderValue, CONTENT_TYPE};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,axum=warn,tower_http=warn"));
    fmt()
        .with_env_filter(filter)
        .compact() // лаконичный формат
        .with_target(false) // без имени таргета
        .init();
    info!("logger initialized");

    // грузим .env из нескольких возможных мест (корень репо / рядом с бинарём)
    _ = ServerConfig::load_dotenvs()?;
    let cfg = ServerConfig::from_env()?;
    cfg.log_summary(); // аккуратно логируем, ничего не паля

    // делимся конфигом со всеми хэндлерами через Extension(Arc<...>)
    let cfg = Arc::new(cfg);

    let pipeline = HybridPipeline::new(cfg.clone()).await?;
    let state = AppState {
        pipeline: Arc::new(pipeline),
    };

    // ⬇️ БЕЗ явной аннотации типа у Router — пусть компилятор сам выведет нужную impl
    let app = Router::new()
        .route("/health", get(health))
        .route("/health/config", get(api::health_config))
        .route(
            "/api/ingest/text_raw",
            post(ingest_text_raw::<HybridPipeline>),
        )
        .route("/api/ingest/text", post(ingest_text::<HybridPipeline>))
        .route("/api/ingest/url", post(ingest_url::<HybridPipeline>))
        .route("/api/ingest/file", post(ingest_file::<HybridPipeline>))
        .route("/api/search", get(search::<HybridPipeline>))
        .route("/api/rag", post(rag::<HybridPipeline>))
        .with_state(state)
        .layer(
            CorsLayer::new()
                .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
                .allow_origin(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .layer(SetResponseHeaderLayer::if_not_present(
            CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=utf-8"),
        ));

    let addr: SocketAddr = format!("{}:{}", cfg.http.bind_addr, cfg.http.bind_port).parse()?;
    tracing::info!("listening on http://{}", addr);

    // Этот вызов в 0.7 корректный
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}
