use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unit {
    pub id: String,
    /// Оригинальный текст (ровно как в исходнике)
    pub text: String,
    pub start_char: usize,
    pub end_char: usize,

    /// Результат рефайна (не затираем original)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refined: Option<String>,

    /// Короткая гиста (1–2 предложения, ~<=30 токенов)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_short: Option<String>,

    /// Длинная гиста (3–5 предложений, ~<=120 токенов)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_long: Option<String>,

    /// Ключевые фразы (3–7 штук)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keyphrases: Vec<String>,

    /// Именованные сущности (как их вернёт LLM — строки)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entities: Vec<String>,

    /// Темы (RU)
    #[serde(default, rename = "topics", skip_serializing_if = "Vec::is_empty")]
    pub topics_ru: Vec<String>,

    /// Темы (EN)
    #[serde(default, rename = "topics_en", skip_serializing_if = "Vec::is_empty")]
    pub topics_en: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic { pub text: String, pub lang: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Composite {
    pub title: String,
    pub summary: String,
    pub topics: Vec<Topic>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SegMode { Auto, LlmFirst, SentencesOnly }
impl Default for SegMode { fn default() -> Self { SegMode::LlmFirst } }

/// Опции LLM-обработки (температура, лимиты и т. п.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefineOptions {
    pub bilingual_topics: bool,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub model: String,
    pub cache_dir: String,
    pub sentence_units: bool,
    #[serde(default)]
    pub seg_mode: SegMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Верхнеуровневая сводка всего документа
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineOutput {
    pub units: Vec<Unit>,
    /// Совместимость со старым форматом: document.summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<Document>,
}
