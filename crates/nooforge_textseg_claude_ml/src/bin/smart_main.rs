// file: src/bin/smart_main.rs
use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use nooforge_textseg_claude_ml::annotator::{Annotator, Marked};
use regex::Regex;
use serde_json::Value as Json; // for merging arrays (если будем объединять)

/// Проверяем/сдвигаем индекс к границе UTF-8 символа (назад).
fn clamp_to_char_boundary(s: &str, mut idx: usize) -> usize {
    if idx > s.len() {
        idx = s.len();
    }
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

/// Базовый целевой размер чанка (символов), по умолчанию 8192, можно переопределить SMART_CHUNK.
fn target_chunk() -> usize {
    std::env::var("SMART_CHUNK")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8192)
}

/// «Умный» поиск ближайшего разделителя возле cut (±pct).
/// Приоритет: пустая строка → заголовок → code fence → подчёркивание → конец предложения → конец строки → пробел (конец слова).
fn pick_smart_cut(text: &str, cut: usize, pct: f32) -> usize {
    let len = text.len();
    if cut >= len {
        return len;
    }

    // окно считаем от целевого размера чанка
    let radius = ((target_chunk() as f32) * pct).round() as isize;

    // старые границы окна по байтам:
    let start = (cut as isize - radius).max(0) as usize;
    let end = (cut as isize + radius).min(len as isize) as usize;

    // ✅ НОВОЕ: прижимаем к границам UTF-8 ПЕРЕД срезом
    let start_b = clamp_to_char_boundary(text, start);
    let end_b = clamp_to_char_boundary(text, end);

    // теперь безопасно
    let window = &text[start_b..end_b];

    let mut best: Option<usize> = None;
    let mut best_score: i32 = -1;
    let mut consider = |pos_in_text: usize, weight: i32| {
        let dist = (pos_in_text as isize - cut as isize).abs() as i32;
        let score = weight - dist;
        if score > best_score {
            best = Some(pos_in_text);
            best_score = score;
        }
    };

    // все индексы из window переводим в координаты исходного текста через start_b
    for (i, _) in window.match_indices("\n\n") {
        consider(start_b + i + 2, 1200);
    }
    for (i, _) in window.match_indices("\n#") {
        consider(start_b + i + 1, 1100);
    }
    for (i, _) in window.match_indices("\n```") {
        consider(start_b + i + 1, 1000);
    }
    for (i, _) in window.match_indices("\n---") {
        consider(start_b + i + 1, 900);
    }
    for (i, _) in window.match_indices("\n===") {
        consider(start_b + i + 1, 900);
    }

    // конец предложения (.!? …)
    let sent_re = Regex::new(r#"[\.!\?](\)|"|\'|\]|\}|>)*(\s|\n)"#).unwrap();
    for m in sent_re.find_iter(window) {
        consider(start_b + m.end(), 800);
    }

    // конец строки / конец слова
    for (i, _) in window.match_indices('\n') {
        consider(start_b + i + 1, 700);
    }
    for (i, _) in window.match_indices(' ') {
        consider(start_b + i + 1, 600);
    }

    // если ничего — вернём ближайшее и опять прижмём к границе символа
    clamp_to_char_boundary(text, best.unwrap_or(cut))
}

/// Первичное разбиение ~ на 4к (или SMART_CHUNK), с «умным» попаданием на разделитель.
fn first_split_spans(text: &str, pct: f32) -> Vec<(usize, usize)> {
    let len = text.len();
    if len == 0 {
        return vec![];
    }
    let chunk = target_chunk();
    let mut spans = Vec::new();
    let mut start = 0usize;
    while start < len {
        let desired = (start + chunk).min(len);
        let end = if desired < len {
            let cut = pick_smart_cut(text, desired, pct);
            cut.max(start + (chunk / 2)).min(len)
        } else {
            len
        };
        let s_b = clamp_to_char_boundary(text, start);
        let e_b = clamp_to_char_boundary(text, end);
        spans.push((s_b, e_b));
        start = e_b;
    }
    spans
}

fn merge_blocks_json(left: &str, right: &str) -> Result<String> {
    let mut l: Vec<Json> = serde_json::from_str::<Vec<Json>>(left).or_else(|_| {
        let v: Json = serde_json::from_str(left)?;
        if let Json::Array(arr) = v {
            Ok(arr)
        } else {
            Err(anyhow!("left blocks not array"))
        }
    })?;
    let r: Vec<Json> = serde_json::from_str::<Vec<Json>>(right).or_else(|_| {
        let v: Json = serde_json::from_str(right)?;
        if let Json::Array(arr) = v {
            Ok(arr)
        } else {
            Err(anyhow!("right blocks not array"))
        }
    })?;
    l.extend(r);
    Ok(serde_json::to_string(&l)?)
}

fn annotate_chunk_recursively(
    annotator: &Annotator,
    base_id: usize,
    text: &str,
    depth: usize,
) -> Result<Marked> {
    let size = text.len();
    let t0 = std::time::Instant::now();
    eprintln!(
        "🔶 [chunk:{}] annotate begin (size={}B depth={})",
        base_id, size, depth
    );
    match annotator.annotate(base_id, text) {
        Ok(m) => {
            eprintln!("✅ [chunk:{}] ok in {:.2?}", base_id, t0.elapsed());
            Ok(m)
        }
        Err(err) => {
            let msg = format!("{err}");
            if (msg.contains("OUTPUT_LIMIT_EXCEEDED")
                || msg.contains("no annotated section found")
                || msg.contains("no blocks json"))
                && size > 2048
                && depth < 4
            {
                eprintln!(
                    "⚠️ [chunk:{}] too large ({}B) — splitting into 2 parts",
                    base_id, size
                );
                let mid_local = clamp_to_char_boundary(text, size / 2);
                let (left, right) = text.split_at(mid_local);
                let left_m =
                    annotate_chunk_recursively(annotator, base_id * 10 + 1, left, depth + 1)?;
                let right_m =
                    annotate_chunk_recursively(annotator, base_id * 10 + 2, right, depth + 1)?;
                // Склеиваем результаты
                let merged_blocks = merge_blocks_json(&left_m.blocks_json, &right_m.blocks_json)?;
                let mut combined = String::with_capacity(
                    left_m.annotated_text.len() + right_m.annotated_text.len(),
                );
                combined.push_str(&left_m.annotated_text);
                combined.push_str(&right_m.annotated_text);
                Ok(Marked {
                    annotated_text: combined,
                    blocks_json: merged_blocks,
                })
            } else {
                eprintln!(
                    "❌ [chunk:{}] annotate error (depth={}): {:#}",
                    base_id, depth, err
                );
                Err(anyhow!(err))
            }
        }
    }
}

fn main() -> Result<()> {
    // Аргумент: путь к файлу
    let mut args = std::env::args_os();
    let _bin = args.next();
    let input: PathBuf = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("usage: smart_main <input_file>"))?;

    // Читаем файл
    let raw = fs::read(&input).with_context(|| format!("read {:?}", &input))?;
    let text = String::from_utf8_lossy(&raw).to_string();

    println!("🚀 file = {}", input.display());
    println!("📦 size = {} bytes", text.as_bytes().len());

    let pct: f32 = std::env::var("SMART_SEARCH_PCT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.10);

    println!(
        "🪓 smart split: chunk={}B search=±{}%",
        target_chunk(),
        (pct * 100.0).round()
    );

    // Первичное разбиение ~4к с умными разделителями
    let spans = first_split_spans(&text, pct);
    let min_sz = spans.iter().map(|(s, e)| e - s).min().unwrap_or(0);
    let max_sz = spans.iter().map(|(s, e)| e - s).max().unwrap_or(0);
    println!("🔪 spans = {}  min={}  max={}", spans.len(), min_sz, max_sz);

    let annotator = Annotator::new()?;

    for (i, (s, e)) in spans.iter().enumerate() {
        let chunk_text = &text[*s..*e];
        println!("🪟 chunk#{} [{}..{}) size={}", i, s, e, e - s);

        let base_id = i + 1;
        match annotate_chunk_recursively(&annotator, base_id, chunk_text, 0) {
            Ok(marked) => {
                let stem = input
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("annotated");
                let mut out_ann = input.clone();
                out_ann.set_file_name(format!("{}__chunk{}_annotated.txt", stem, i));
                let mut out_json = input.clone();
                out_json.set_file_name(format!("{}__chunk{}_blocks.json", stem, i));

                // ⚠️ НЕ сериализуем annotated_text в JSON и не печатаем через {:?}
                // (именно это превращает \n в буквальный backslash+n)

                // (опционально) на Windows можно форсировать CRLF, чтобы старые редакторы
                // показывали переносы корректно: включи ANNOTATED_CRLF=1
                let annotated_to_save = if cfg!(windows)
                    && std::env::var("ANNOTATED_CRLF").ok().as_deref() == Some("1")
                {
                    // нормализуем сначала на LF, затем LF → CRLF
                    marked
                        .annotated_text
                        .replace("\r\n", "\n")
                        .replace('\n', "\r\n")
                } else {
                    marked.annotated_text
                };

                // ✅ пишем как обычный текст/байты — получатся настоящие переводы строк
                std::fs::write(&out_ann, annotated_to_save.as_bytes())
                    .with_context(|| format!("write annotated to {}", out_ann.display()))?;

                // JSON с метаданными мы пишем отдельно (это ок, так и нужно)
                std::fs::write(&out_json, marked.blocks_json.as_bytes())
                    .with_context(|| format!("write blocks json to {}", out_json.display()))?;

                println!("💾 saved: {}, {}", out_ann.display(), out_json.display());

                // std::fs::write(&out_ann, marked.annotated_text.as_bytes())
                //     .with_context(|| format!("write annotated to {}", out_ann.display()))?;
                // std::fs::write(&out_json, marked.blocks_json.as_bytes())
                //     .with_context(|| format!("write blocks json to {}", out_json.display()))?;

                // println!("💾 saved: {}, {}", out_ann.display(), out_json.display());
            }
            Err(err) => {
                eprintln!("❌ [chunk:{}] failed: {:#}", i, err);
                return Err(err);
            }
        }
    }

    Ok(())
}
