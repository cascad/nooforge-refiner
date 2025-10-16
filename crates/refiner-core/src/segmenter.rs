use crate::types::Unit;
use regex::Regex;

/// Простая сегментация на предложения (по . ! ? …)
/// Возвращает вектор Unit с корректными координатами исходного текста.
/// ВАЖНО: `text` — это оригинальный фрагмент; все новые поля заполняем дефолтами.
pub fn split_sentences(text: &str) -> Vec<Unit> {
    let rx = Regex::new(r"(?s)(.*?)([.!?…]+|$)").unwrap();
    let mut out = Vec::new();

    for (i, cap) in rx.captures_iter(text).enumerate() {
        let whole = cap.get(0).unwrap();   // полный матч в исходнике
        let raw = whole.as_str();          // с пробелами/переносами как есть
        let trimmed = raw.trim();

        if trimmed.is_empty() {
            continue;
        }

        // Компенсация trim: возвращаем координаты в рамках исходного текста
        let left_trim = raw.len() - raw.trim_start().len();
        let right_trim = raw.len() - raw.trim_end().len();
        let start = whole.start() + left_trim;
        let end = whole.end() - right_trim;

        out.push(Unit {
            id: format!("u{:05}", i),
            text: trimmed.to_string(),
            start_char: start,
            end_char: end,

            // новые поля — дефолтные значения
            refined: None,
            summary_short: None,
            summary_long: None,
            keyphrases: Vec::new(),
            entities: Vec::new(),
            topics_ru: Vec::new(),
            topics_en: Vec::new(),
        });
    }

    out
}
