use crate::types::Unit;
use regex::Regex;

/// Простая сегментация на предложения (по . ! ? …)
/// возвращает вектор Unit с корректными координатами исходного текста.
pub fn split_sentences(text: &str) -> Vec<Unit> {
    let rx = Regex::new(r"(?s)(.*?)([.!?…]+|$)").unwrap();
    let mut out = Vec::new();

    for (i, cap) in rx.captures_iter(text).enumerate() {
        let whole = cap.get(0).unwrap(); // полный матч
        let raw = whole.as_str(); // как есть, включая пробелы
        let trimmed = raw.trim();

        if trimmed.is_empty() {
            continue;
        }

        // вычисляем смещения, чтобы после trim координаты указывали
        // на исходный диапазон внутри оригинального текста
        let left_trim = raw.len() - raw.trim_start().len();
        let right_trim = raw.len() - raw.trim_end().len();

        let start = whole.start() + left_trim;
        let end = whole.end() - right_trim;

        out.push(Unit {
            id: format!("u{:05}", i),
            text: trimmed.to_string(),
            start_char: start,
            end_char: end,
        });
    }

    out
}
