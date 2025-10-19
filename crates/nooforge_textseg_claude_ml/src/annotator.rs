// file: src/annotator.rs
use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use regex::Regex;
use reqwest::blocking::Client;
use serde_json::json;

/// Результат разметки
#[derive(Debug, Clone)]
pub struct Marked {
    /// Аннотированный текст с inline-маркерами [[[BLOCK id=... kind=... summary:`...` tags:`...`]]]
    pub annotated_text: String,
    /// Сгенерированный у нас JSON по маркерам (id/kind/summary/tags) — ДЛЯ INGEST (модель JSON не присылает)
    pub blocks_json: String,
}

/// Аннотатор, который ходит в OpenRouter (Chat Completions)
#[derive(Clone)]
pub struct Annotator {
    pub client: Client,
    pub model: String,
}

impl Annotator {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(60))
            .pool_idle_timeout(std::time::Duration::from_secs(15))
            .build()?;

        let model = std::env::var("OPENROUTER_MODEL")
            .unwrap_or_else(|_| "qwen/qwen-2.5-72b-instruct".to_string());

        Ok(Self { client, model })
    }

    /// Аннотируем кусок: просим ТОЛЬКО annotated-блок, парсим маркеры и строим JSON локально.
    pub fn annotate(&self, base_id: usize, text: &str) -> Result<Marked> {
        let key = std::env::var("OPENROUTER_API_KEY").context("OPENROUTER_API_KEY is not set")?;

        let sys: &str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/prompts/annotator_system_v5_markers.txt"
        ));
        let fewshot: &str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/prompts/annotator_fewshot_v5_markers.txt"
        ));

        let user = format!(
            "Annotate the following text exactly as per the system rules and example. Input:\n```TEXT\n{}\n```",
            text
        );

        let max_output_tokens: usize = std::env::var("OPENROUTER_MAX_TOKENS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(8192);

        let payload = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role":"system","content": sys},
                {"role":"user","content": fewshot},
                {"role":"user","content": user},
            ],
            "temperature": 0.0,
            "max_tokens": max_output_tokens
        });

        // ── ЛОГИ ─────────────────────────────────────────────────────────────────
        let payload_len = serde_json::to_string(&payload)
            .map(|s| s.len())
            .unwrap_or(0);
        eprintln!(
            "🔶 [annotate] base_id={} text_len={}B payload_len={}B max_tokens={}",
            base_id,
            text.len(),
            payload_len,
            max_output_tokens
        );

        eprintln!("⏳ [net] sending request…");
        let t_all = Instant::now();
        let t_send = Instant::now();
        let resp = self
            .client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .bearer_auth(&key)
            .header("HTTP-Referer", "https://nooforge.local")
            .header("X-Title", "nooforge-minmark-seg-chunk")
            .json(&payload)
            .send()
            .context("reqwest send")?;
        eprintln!("✅ [net] headers received in {:.2?}", t_send.elapsed());

        eprintln!("⏳ [net] reading body…");
        let t_body = Instant::now();
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        eprintln!(
            "✅ [net] body read in {:.2?} (status={}, body_len={}B)",
            t_body.elapsed(),
            status,
            body.len()
        );

        if !status.is_success() {
            return Err(anyhow!(
                "openrouter error: HTTP {}. Body (first 800): {}",
                status,
                body.chars().take(800).collect::<String>()
            ));
        }
        if body.trim() == "OUTPUT_LIMIT_EXCEEDED" {
            return Err(anyhow!(
                "model indicated OUTPUT_LIMIT_EXCEEDED — reduce chunk size"
            ));
        }

        eprintln!("⏳ [parse] extracting annotated block…");
        let re_ann = Regex::new(r"(?s)<<<BEGIN_ANNOTATED>>>\s*(.*?)\s*<<<END_ANNOTATED>>>")?;
        let annotated = re_ann
            .captures(&body)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| anyhow!("no annotated block found"))?;
        let annotated = maybe_unescape_annotated(&annotated);
        eprintln!("✅ [parse] annotated extracted (len={}B)", annotated.len());

        // Разбираем маркеры и строим JSON (локально)
        eprintln!("⏳ [meta] parsing markers to JSON…");
        let (meta_json, open_count, close_count) = markers_to_json(&annotated)?;
        eprintln!(
            "✅ [meta] markers ok (open={} close={})",
            open_count, close_count
        );

        let blocks_json = if std::env::var("BLOCKS_PRETTY").ok().as_deref() == Some("1") {
            serde_json::to_string_pretty(&meta_json)?
        } else {
            serde_json::to_string(&meta_json)?
        };

        eprintln!("🟢 [annotate] done in {:.2?}", t_all.elapsed());
        Ok(Marked {
            annotated_text: annotated,
            blocks_json,
        })
    }
}

/// Парсим маркеры вида:
/// [[[BLOCK id=1 kind=Text summary:`...` tags:`a,b,c`]]]
/// [[[/BLOCK]]]
fn markers_to_json(annotated: &str) -> Result<(serde_json::Value, usize, usize)> {
    let mut stack: Vec<i64> = Vec::new();
    let mut items: Vec<serde_json::Value> = Vec::new();

    // Опциональность поля kind/summary/tags поддерживаем, id обязателен
    let open_re = Regex::new(
        r#"\[\[\[BLOCK\s+id=(\d+)(?:\s+kind=([A-Za-z][A-Za-z0-9_]*))?(?:\s+summary:`([^`]*)`)?(?:\s+tags:`([^`]*)`)?\s*\]\]\]"#,
    )?;
    let close_re = Regex::new(r#"\[\[\[/BLOCK\]\]\]"#)?;

    let mut open_count = 0usize;
    let mut close_count = 0usize;

    // Сканируем по тексту слева направо, находим все маркеры в порядке появления
    // и поддерживаем стек для простейшей проверки вложенности.
    let mut i = 0usize;
    while i < annotated.len() {
        if let Some(m) = open_re.find_at(annotated, i) {
            // если нашли закрывающий раньше — обработаем его
            if let Some(c) = close_re.find_at(annotated, i) {
                if c.start() < m.start() {
                    // закрывающий без явного id — просто уменьшаем стек
                    if stack.pop().is_none() {
                        return Err(anyhow!("unbalanced closing marker at byte {}", c.start()));
                    }
                    close_count += 1;
                    i = c.end();
                    continue;
                }
            }
            // обработка открытия
            let caps = open_re.captures(&annotated[m.start()..m.end()]).unwrap();
            let id: i64 = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
            let kind = caps.get(2).map(|x| x.as_str().to_string());
            let summary = caps.get(3).map(|x| x.as_str().to_string());
            let tags = caps
                .get(4)
                .map(|x| x.as_str().to_string())
                .unwrap_or_default();

            stack.push(id);
            open_count += 1;

            let tags_vec: Vec<String> = tags
                .split(',')
                .map(|t| t.trim())
                .filter(|t| !t.is_empty())
                .map(|t| t.to_string())
                .collect();

            let mut obj = json!({
                "id": id,
                "tags": tags_vec,
            });
            if let Some(k) = kind {
                obj["kind"] = json!(k);
            }
            if let Some(s) = summary {
                obj["summary"] = json!(s);
            }

            items.push(obj);
            i = m.end();
            continue;
        }
        if let Some(c) = close_re.find_at(annotated, i) {
            if stack.pop().is_none() {
                return Err(anyhow!("unbalanced closing marker at byte {}", c.start()));
            }
            close_count += 1;
            i = c.end();
            continue;
        }
        // ничего не нашли — двигаем по байту
        i += 1;
    }

    if !stack.is_empty() {
        return Err(anyhow!("unbalanced markers: {} not closed", stack.len()));
    }

    Ok((json!(items), open_count, close_count))
}

fn maybe_unescape_annotated(s: &str) -> String {
    // Consider escaped if there are many literal sequences and few real newlines,
    // or the text starts with "\n[[[" which models sometimes emit.
    let real_nl = s.matches('\n').count();
    let lit_nl = s.matches("\\\\n").count();
    let lit_crnl = s.matches("\\\\r\\\\n").count();
    let lit_tab = s.matches("\\\\t").count();
    let looks_escaped = lit_nl + lit_crnl + lit_tab > real_nl.saturating_mul(2)
        || (lit_nl > 10 && real_nl < 5)
        || s.trim_start().starts_with("\\n[[[");

    if looks_escaped {
        let mut out = s.to_string();
        out = out.replace("\\r\\n", "\n");
        out = out.replace("\\n", "\n");
        out = out.replace("\\t", "\t");
        out = out.replace("\\\"", "\"");
        out = out.replace("\\\\", "\\");
        out
    } else {
        s.to_string()
    }
}
