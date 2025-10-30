use reqwest::multipart;
use serde_json::json;
use std::path::PathBuf;

const API_URL: &str = "http://127.0.0.1:8090";

pub async fn ingest_text_api(text: String) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())?;

    let body = json!({ "text": text });

    let resp = client
        .post(format!("{}/api/ingest/text", API_URL))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let text = resp.text().await.map_err(|e| e.to_string())?;
    Ok(crate::utils::strip_ansi_codes(&text))
}

pub async fn ingest_file_api(path: PathBuf) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())?;

    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| format!("Read file: {}", e))?;
    let filename = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("file")
        .to_string();
    let part = multipart::Part::bytes(bytes).file_name(filename);
    let form = multipart::Form::new().part("file", part);

    let resp = client
        .post(format!("{}/api/ingest/file", API_URL))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let text = resp.text().await.map_err(|e| e.to_string())?;
    Ok(crate::utils::strip_ansi_codes(&text))
}

pub async fn rag_api(query: String, limit: usize) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())?;

    let body = json!({ "q": query, "limit": limit });

    let resp = client
        .post(format!("{}/api/rag", API_URL))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let text = resp.text().await.map_err(|e| e.to_string())?;
    Ok(crate::utils::strip_ansi_codes(&text))
}

pub async fn search_api(query: String, limit: usize) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(format!("{}/api/search", API_URL))
        .query(&[("q", query), ("limit", limit.to_string())])
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let text = resp.text().await.map_err(|e| e.to_string())?;
    Ok(crate::utils::strip_ansi_codes(&text))
}

pub async fn pick_file() -> Option<std::path::PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_title("Pick a file")
        .pick_file()
        .await
        .map(|f| f.path().to_path_buf())
}
