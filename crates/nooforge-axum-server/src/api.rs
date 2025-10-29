// /src/api.rs
use axum::{
    extract::{Multipart, Query, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::pipeline::Pipeline;
use crate::{
    model::{IngestResult, RagRequest, RagResponse, SearchQuery, SearchResult},
    pipeline::HasConfig,
};

use axum::{
    async_trait,
    extract::{FromRequest, Request},
};
use bytes::Bytes;
use chardetng::EncodingDetector;
use encoding_rs::{Encoding, UTF_16BE, UTF_16LE};
use http_body_util::BodyExt;

// Универсальный экстрактор JSON из "любых байт" (любая кодировка → UTF-8 → serde_json)
pub struct JsonAnyEncoding<T>(pub T);

pub struct JsonUtf<T>(pub T);

impl<T: Serialize> IntoResponse for JsonUtf<T> {
    fn into_response(self) -> Response {
        // сериализуем в UTF-8 байты
        let body = match serde_json::to_vec(&self.0) {
            Ok(v) => v,
            Err(e) => {
                // если вдруг ошибка сериализации — вернём 500
                return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                    .into_response();
            }
        };
        let mut resp = body.into_response();
        // жёстко укажем charset
        resp.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=utf-8"),
        );
        resp
    }
}

#[async_trait]
impl<S, T> FromRequest<S> for JsonAnyEncoding<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned + Send,
{
    type Rejection = (axum::http::StatusCode, String);

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        // читаем сырое тело (bytes)
        let bytes = Bytes::from_request(req, state).await.map_err(|e| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                format!("read body: {e}"),
            )
        })?;

        // детект кодировки → UTF-8
        let mut det = EncodingDetector::new();
        det.feed(&bytes, true);
        let enc = det.guess(None, true);
        let (cow, _, _) = enc.decode(&bytes);
        let s = cow.into_owned();

        // парсим JSON
        let val = serde_json::from_str::<T>(&s).map_err(|e| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                format!("json parse: {e}"),
            )
        })?;
        Ok(JsonAnyEncoding(val))
    }
}

pub struct AppState<P> {
    pub pipeline: Arc<P>,
}

// Ручная реализация Clone — клонируем только Arc, без требований к P.
impl<P> Clone for AppState<P> {
    fn clone(&self) -> Self {
        Self {
            pipeline: Arc::clone(&self.pipeline),
        }
    }
}

#[derive(Deserialize)]
pub struct IngestTextReq {
    pub text: String,
    pub lang: Option<String>,
    pub title: Option<String>,
    pub explain: Option<bool>,
}

#[derive(Deserialize)]
pub struct IngestUrlReq {
    pub url: String,
    pub lang: Option<String>,
    pub title: Option<String>,
}

#[derive(Deserialize)]
pub struct IngestRawQuery {
    pub title: Option<String>,
    pub lang: Option<String>,
    pub explain: Option<bool>,
}

pub async fn ingest_text<P>(
    State(st): State<AppState<P>>,
    JsonAnyEncoding(req): JsonAnyEncoding<IngestTextReq>,
) -> Result<JsonUtf<IngestResult>, (StatusCode, String)>
where
    P: Pipeline + Send + Sync + 'static,
{
    // дальше твой прежний код:
    // st.pipeline.ingest_text(req.text, req.lang, req.title, req.explain).await ...
    st.pipeline
        .ingest_text(req.text, req.lang, req.title, req.explain)
        .await
        .map(JsonUtf)
        .map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e.to_string()))
}

pub async fn ingest_url<P>(
    State(st): State<AppState<P>>,
    Json(req): Json<IngestUrlReq>,
) -> Result<JsonUtf<IngestResult>, (StatusCode, String)>
where
    P: Pipeline + Send + Sync + 'static,
{
    st.pipeline
        .ingest_url(req.url, req.lang, req.title)
        .await
        .map(JsonUtf)
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))
}

pub async fn ingest_file<P>(
    State(st): State<AppState<P>>,
    mut mp: Multipart,
) -> Result<JsonUtf<IngestResult>, (StatusCode, String)>
where
    P: Pipeline + Send + Sync + 'static,
{
    let mut file_name = None;
    let mut file_bytes: Vec<u8> = vec![];
    let mut lang = None;
    let mut title = None;

    while let Some(field) = mp
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            file_name = field
                .file_name()
                .map(|s| s.to_string())
                .or(Some("upload.bin".into()));
            file_bytes = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
                .to_vec();
        } else if name == "lang" {
            let lb = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
                .to_vec();
            lang = Some(decode_to_utf8_hard(&lb));
        } else if name == "title" {
            let tb = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
                .to_vec();
            title = Some(decode_to_utf8_hard(&tb));
        }
    }

    let name = file_name.unwrap_or_else(|| "upload.bin".into());
    st.pipeline
        .ingest_file(name, file_bytes, lang, title)
        .await
        .map(JsonUtf)
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))
}

pub async fn search<P>(
    State(st): State<AppState<P>>,
    Query(q): Query<SearchQuery>,
) -> Result<JsonUtf<SearchResult>, (StatusCode, String)>
where
    P: Pipeline + Send + Sync + 'static,
{
    st.pipeline
        .search_hybrid(q.q, q.only_latest, q.limit)
        .await
        .map(JsonUtf)
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))
}

pub async fn rag<P>(
    State(st): State<AppState<P>>,
    Json(req): Json<RagRequest>,
) -> Result<JsonUtf<RagResponse>, (StatusCode, String)>
where
    P: Pipeline + Send + Sync + 'static,
{
    st.pipeline
        .rag_answer(
            req.q,
            req.limit,
            req.stream,
            req.model,
            req.temperature,
            req.max_tokens,
        )
        .await
        .map(JsonUtf)
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))
}

pub async fn health() -> &'static str {
    "ok"
}

pub async fn health_config<P>(
    State(st): State<AppState<P>>,
) -> Result<JsonUtf<serde_json::Value>, (StatusCode, String)>
where
    P: Pipeline + HasConfig + Send + Sync + 'static,
{
    // Arc<P> авто-дирефнётся до P, если трейт в скоупе
    let masked = st.pipeline.config().reveal_masked();
    Ok(JsonUtf(json!({ "config": masked })))
}

/// Жёсткая декодировка любых байтов в UTF-8:
/// 1) BOM → используем её;
/// 2) пробуем как UTF-8 напрямую;
/// 3) эвристика UTF-16 без BOM (по нулям) → LE/BE;
/// 4) chardet → encoding_rs;
fn decode_to_utf8_hard(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }

    // 1) BOM?
    if let Some((enc, off)) = Encoding::for_bom(bytes) {
        let (cow, _, _) = enc.decode(&bytes[off..]);
        return cow.into_owned();
    }

    // 2) Чистый UTF-8?
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }

    // 3) Эвристика UTF-16 без BOM (много нулей)
    let nul_ratio = bytes.iter().filter(|&&b| b == 0).count() as f32 / bytes.len() as f32;
    if nul_ratio > 0.2 {
        // Определим эндиянность: если bytes[0]==0 и bytes[1]!=0 — похоже на BE, иначе LE
        let enc =
            if bytes.get(0).copied().unwrap_or(0) == 0 && bytes.get(1).copied().unwrap_or(1) != 0 {
                UTF_16BE
            } else {
                UTF_16LE
            };
        let (cow, _, _) = enc.decode(bytes);
        return cow.into_owned();
    }

    // 4) Детект кодировки → decode
    let mut det = EncodingDetector::new();
    det.feed(bytes, true);
    let enc = det.guess(None, true);
    let (cow, _, _) = enc.decode(bytes);
    cow.into_owned()
}

fn normalize_text(s: &str) -> String {
    let mut out = s.replace("\r\n", "\n").replace('\r', "\n");
    // опционально подчистим нули/непечатаемые границы
    out = out.trim_matches('\u{FEFF}').trim_matches('\0').to_string();
    out
}

/// POST /api/ingest/text_raw?title=...&lang=...
/// Тело запроса — любые байты; внутри приводим всё к UTF-8 и индексируем.
pub async fn ingest_text_raw<P>(
    State(st): State<AppState<P>>,
    Query(q): Query<IngestRawQuery>,
    mut req: Request, // <- берём сырой Request вместо Bytes
) -> Result<JsonUtf<IngestResult>, (StatusCode, String)>
where
    P: Pipeline + Send + Sync + 'static,
{
    // Надёжно собираем body (включая chunked)
    let collected = req
        .body_mut()
        .collect()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("read body: {e}")))?;
    let bytes = collected.to_bytes();

    if bytes.is_empty() {
        // Чёткая ошибка — чтобы сразу понять источник
        return Err((
            StatusCode::BAD_REQUEST,
            "ingest_text_raw: empty request body (0 bytes)".to_string(),
        ));
    }

    let text = {
        let text = decode_to_utf8_hard(&bytes);
        normalize_text(&text)
    };

    tracing::info!(
        "ingest_text_raw: len_bytes={}, len_text={}",
        bytes.len(),
        text.len()
    );

    tracing::info!("first 32 bytes hex: {:02X?}", &bytes[..bytes.len().min(32)]);
    tracing::info!("escaped: {:?}", text);

    st.pipeline
        .ingest_text(text, q.lang, q.title, q.explain)
        .await
        .map(JsonUtf)
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))
}
