use clap::{Parser, ValueEnum};
use tracing_subscriber::EnvFilter;
use refiner_core::types::SegMode;


#[derive(Copy, Clone, Eq, PartialEq, ValueEnum, Debug)]
enum SegArg { Auto, LlmFirst, SentencesOnly }


impl From<SegArg> for SegMode {
fn from(v: SegArg) -> Self {
match v { SegArg::Auto => SegMode::Auto, SegArg::LlmFirst => SegMode::LlmFirst, SegArg::SentencesOnly => SegMode::SentencesOnly }
}
}


#[derive(Parser, Debug)]
#[command(name = "refiner")]
#[command(about = "NooForge Refiner — Rust CLI")]
struct Args {
/// e.g. qwen/qwen-2.5-72b-instruct
#[arg(long, default_value = "qwen/qwen-2.5-72b-instruct")]
model: String,


/// path to input text file
#[arg(long)]
input: String,


/// path to output JSON file
#[arg(long, default_value = "out.json")]
output: String,


/// segmentation mode
#[arg(long, value_enum, default_value_t = SegArg::LlmFirst)]
seg_mode: SegArg,


/// enable sentence-based units
#[arg(long, default_value_t = true)]
sentence_units: bool,


/// cache dir
#[arg(long, default_value = ".cache/nooforge_llm")]
cache: String,


/// log level
#[arg(long, default_value = "info")]
log_level: String,
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
let args = Args::parse();
tracing_subscriber::fmt()
.with_env_filter(EnvFilter::new(args.log_level.clone()))
.with_target(false)
.compact()
.init();


let api_key = std::env::var("OPENROUTER_API_KEY").expect("Set OPENROUTER_API_KEY env var");


let llm = llm_openrouter::OpenRouterClient::new(api_key);


let cfg = refiner_core::types::PipelineConfig {
model: args.model.clone(),
cache_dir: args.cache.clone(),
sentence_units: args.sentence_units,
seg_mode: args.seg_mode.into(),
};


let input_text = tokio::fs::read_to_string(&args.input).await?;
let out = refiner_core::pipeline::run(&llm, &cfg, &input_text).await?;
let json = serde_json::to_string_pretty(&out)?;
tokio::fs::write(&args.output, json).await?;
Ok(())
}