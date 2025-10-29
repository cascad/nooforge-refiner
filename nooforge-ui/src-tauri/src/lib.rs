// nooforge-ui/src-tauri/src/lib.rs
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct IngestTextReq {
    pub text: String,
    pub lang: Option<String>,
    pub title: Option<String>,
    pub explain: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct IngestFileReq {
    pub name: String,
    pub data_b64: String,
    pub lang: Option<String>,
    pub title: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RagReq {
    pub q: String,
    pub limit: Option<usize>,
    pub stream: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct SearchReq {
    pub q: String,
    pub limit: Option<usize>,
}

pub mod commands {
    use super::*;
    use base64::Engine;
    use reqwest::header;

    pub async fn ingest_text(req: IngestTextReq) -> Result<serde_json::Value, String> {
        let client = reqwest::Client::new();
        let lang = req.lang.unwrap_or_else(|| "ru".into());
        let title = req.title.unwrap_or_default();
        let explain = req.explain.unwrap_or(false);

        let url = format!(
            "http://127.0.0.1:8090/api/ingest/text_raw?lang={}&title={}&explain={}",
            urlencoding::encode(&lang),
            urlencoding::encode(&title),
            explain
        );

        let bytes = req.text.into_bytes();
        let resp = client
            .post(url)
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(bytes)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        resp.json::<serde_json::Value>()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn ingest_file(req: IngestFileReq) -> Result<serde_json::Value, String> {
        let client = reqwest::Client::new();
        let lang = req.lang.unwrap_or_else(|| "ru".into());
        let title = req.title.unwrap_or_default();

        // исправлено предупреждение: используем новый API base64
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(req.data_b64.as_bytes())
            .map_err(|e| e.to_string())?;

        let part = reqwest::multipart::Part::bytes(bytes).file_name(req.name);

        let form = reqwest::multipart::Form::new()
            .text("lang", lang)
            .text("title", title)
            .part("file", part);

        let resp = client
            .post("http://127.0.0.1:8090/api/ingest/file")
            .multipart(form)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        resp.json::<serde_json::Value>()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn rag(req: RagReq) -> Result<serde_json::Value, String> {
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "q": req.q,
            "limit": req.limit.unwrap_or(6),
            // "stream": req.stream.unwrap_or(false),
        });

        let resp = client
            .post("http://127.0.0.1:8090/api/rag")
            .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        println!("shit! -> {:?}", resp);

        resp.json::<serde_json::Value>()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn search(req: SearchReq) -> Result<serde_json::Value, String> {
        let client = reqwest::Client::new();
        let url = format!(
            "http://127.0.0.1:8090/api/search?q={}&limit={}",
            urlencoding::encode(&req.q),
            req.limit.unwrap_or(10)
        );

        let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
        resp.json::<serde_json::Value>()
            .await
            .map_err(|e| e.to_string())
    }
}
