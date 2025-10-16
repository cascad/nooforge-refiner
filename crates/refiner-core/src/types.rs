use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unit {
pub id: String,
pub text: String,
pub start_char: usize,
pub end_char: usize,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic { pub text: String, pub lang: String }


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Composite {
pub title: String,
pub summary: String,
pub topics: Vec<Topic>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefineOptions {
pub bilingual_topics: bool,
pub temperature: f32,
pub max_tokens: Option<u32>,
}


#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SegMode { Auto, LlmFirst, SentencesOnly }


impl Default for SegMode { fn default() -> Self { SegMode::LlmFirst } }


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
pub model: String,
pub cache_dir: String,
pub sentence_units: bool,
#[serde(default)]
pub seg_mode: SegMode,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineOutput {
pub units: Vec<Unit>,
pub composite: Option<Composite>,
}