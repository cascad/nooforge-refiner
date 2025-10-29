// nooforge-ui/src-tauri/src/main.rs
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use std::time::Duration;

use reqwest::{header, multipart, Client};

const BASE_URL: &str = "http://127.0.0.1:8090";

fn http_client() -> Result<Client, String> {
    reqwest::Client::builder()
        .no_proxy()          // важно: не используем системный прокси для localhost
        .http1_only()        // надёжнее для локалки
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn rag(q: String, limit: Option<usize>) -> Result<String, String> {
    let client = http_client()?;
    let body = serde_json::json!({
        "q": q,
        "limit": limit.unwrap_or(5)
    });

    let resp = client
        .post(format!("{BASE_URL}/api/rag"))
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let text = resp.text().await.map_err(|e| e.to_string())?;
    Ok(text)
}

#[tauri::command]
async fn search(q: String, limit: Option<usize>) -> Result<String, String> {
    let client = http_client()?;
    let url = format!("{BASE_URL}/api/search");
    let resp = client
        .get(url)
        .query(&[("q", q), ("limit", limit.unwrap_or(10).to_string())])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let text = resp.text().await.map_err(|e| e.to_string())?;
    Ok(text)
}

#[tauri::command]
async fn ingest_text(text: String) -> Result<String, String> {
    // сервер ждёт JSON с полем `text`
    let client = http_client()?;
    let body = serde_json::json!({ "text": text });

    let resp = client
        .post(format!("{BASE_URL}/api/ingest/text"))
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let text = resp.text().await.map_err(|e| e.to_string())?;
    Ok(text)
}

#[tauri::command]
async fn ingest_file(path: String) -> Result<String, String> {
    let client = http_client()?;

    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| format!("read file failed: {e}"))?;

    let filename = std::path::Path::new(&path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("file.bin")
        .to_string();

    let part = multipart::Part::bytes(bytes).file_name(filename);
    let form = multipart::Form::new().part("file", part);

    let resp = client
        .post(format!("{BASE_URL}/api/ingest/file"))
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let text = resp.text().await.map_err(|e| e.to_string())?;
    Ok(text)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            rag,
            search,
            ingest_text,
            ingest_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
