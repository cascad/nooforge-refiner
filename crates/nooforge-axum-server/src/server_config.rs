// file: src/server_config.rs
use sha2::{Digest, Sha256};
use std::env;

type AnyResult<T> = anyhow::Result<T>;

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub llm: LlmSection,
    pub http: HttpSection,
    pub ingest: IngestSection,
    pub search: SearchSection,
    pub hybrid: HybridSection,
}

#[derive(Clone, Debug)]
pub struct LlmSection {
    pub provider: String,        // "openrouter" | ...
    pub base_url: String,        // OpenRouter chat completions URL
    pub api_key: Option<String>, // нормализованный ключ (может отсутствовать)
    pub model_primary: String,
    pub model_fallbacks: Vec<String>,
    pub timeout_secs: u64,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct HttpSection {
    pub bind_addr: String,
    pub bind_port: u16,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct IngestSection {
    pub default_lang: String,
    pub explain: bool,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct SearchSection {
    pub limit_default: usize,
    pub limit_max: usize,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct HybridSection {
    pub model_dir: String,      // из HYBRID_MODEL_DIR
    pub tokenizer_path: String, // из HYBRID_TOKENIZER_PATH
    pub max_tokens: usize,      // для чанкинга
    pub overlap_tokens: usize,  // для чанкинга
    pub qdrant_host: String,
    pub qdrant_port: u16,
    pub qdrant_collection: String,
    pub source_prefix: String,
}

impl ServerConfig {
    pub fn load_dotenvs() -> anyhow::Result<()> {
        use std::path::Path;

        fn load_one(path: &Path) -> anyhow::Result<()> {
            match dotenvy::from_filename(path) {
                Ok(_) => {
                    let absolute_path = std::fs::canonicalize(path)?;
                    tracing::info!(
                        "loaded .env from {}: {}",
                        path.display(),
                        absolute_path.display()
                    );
                    Ok(())
                }
                Err(dotenvy::Error::Io(_)) => {
                    // файла нет — просто предупреждаем
                    tracing::warn!("no .env at {}", path.display());
                    Ok(())
                }
                Err(e) => {
                    // файл есть, но **битый → падаем**
                    tracing::error!("❌ malformed .env at {}: {}", path.display(), e);
                    Err(anyhow::anyhow!("Malformed .env file: {}", path.display()))
                }
            }
        }

        // пробуем только корневой .env (не ищем по родителям, без магии)
        load_one(Path::new(".env"))
    }

    pub fn from_env() -> AnyResult<Self> {
        // --- LLM ---
        let raw_key = env::var("OPENROUTER_API_KEY").ok();
        let api_key = raw_key
            .as_ref()
            .map(|s| normalize_secret(s))
            .filter(|s| !s.is_empty());
        if let Some(ref k) = api_key {
            validate_api_key(k)?;
        }

        let llm = LlmSection {
            provider: get_env_or_warn("LLM_PROVIDER", "openrouter"),
            base_url: get_env_or_warn("LLM_BASE_URL", "https://openrouter.ai/api/v1"),
            api_key: Some(require_secret("OPENROUTER_API_KEY")?), // ← тут ПАДАЕМ если нет
            model_primary: get_env_or_warn("LLM_MODEL", "qwen/qwen-2.5-7b-instruct"),
            model_fallbacks: get_env_or_warn("LLM_FALLBACK_MODELS", "")
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            timeout_secs: get_env_num_or_warn("LLM_TIMEOUT_SECS", 45),
            max_tokens: get_env_num_or_warn("LLM_MAX_TOKENS", 512),
            temperature: get_env_num_or_warn("LLM_TEMPERATURE", 0.2),
        };

        // --- HTTP ---
        let http = HttpSection {
            bind_addr: get_env_or_warn("BIND_ADDR", "127.0.0.1"),
            bind_port: get_env_num_or_warn("BIND_PORT", 8090),
        };

        // --- Ingest ---
        let ingest = IngestSection {
            default_lang: env::var("INGEST_DEFAULT_LANG").unwrap_or_else(|_| "ru".to_string()),
            explain: env::var("INGEST_EXPLAIN")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
        };

        // --- Search ---
        let search = SearchSection {
            limit_default: env::var("SEARCH_LIMIT_DEFAULT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            limit_max: env::var("SEARCH_LIMIT_MAX")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(50),
        };

        // --- Hybrid (ВАЖНО: сохраняем старые ENV имена) ---
        let hybrid = HybridSection {
            model_dir: get_env_or_warn("HYBRID_MODEL_DIR", "./models/multilingual-e5-base"),
            tokenizer_path: get_env_or_warn(
                "HYBRID_TOKENIZER_PATH",
                "./models/multilingual-e5-base/tokenizer.json",
            ),
            qdrant_host: get_env_or_warn("QDRANT_HOST", "127.0.0.1"),
            qdrant_port: get_env_num_or_warn("QDRANT_PORT", 6334),
            qdrant_collection: get_env_or_warn("QDRANT_COLLECTION", "chunks"),
            source_prefix: get_env_or_warn("HYBRID_SOURCE_PREFIX", "file://"),
            max_tokens: get_env_num_or_warn("HYBRID_CHUNK_MAX_TOKENS", 350),
            overlap_tokens: get_env_num_or_warn("HYBRID_CHUNK_OVERLAP", 60),
        };

        Ok(Self {
            llm,
            http,
            ingest,
            search,
            hybrid,
        })
    }

    pub fn log_summary(&self) {
        if let Some(k) = &self.llm.api_key {
            let len = k.len();
            let hash = sha256_8(k);
            tracing::info!("✅ LLM api_key: len={}, sha256[:8]={}", len, hash);
        } else {
            tracing::warn!("⚠️  LLM api_key not set");
        }
        tracing::info!(
          "LLM provider='{}' base_url={} model='{}' fallbacks={:?} timeout={}s max_tokens={} temp={}",
          self.llm.provider, self.llm.base_url, self.llm.model_primary, self.llm.model_fallbacks,
          self.llm.timeout_secs, self.llm.max_tokens, self.llm.temperature
        );
        tracing::info!("HTTP {}:{}", self.http.bind_addr, self.http.bind_port);
        tracing::info!(
            "Hybrid model_dir='{}' tokenizer='{}' chunk.max={} chunk.overlap={}",
            self.hybrid.model_dir,
            self.hybrid.tokenizer_path,
            self.hybrid.max_tokens,
            self.hybrid.overlap_tokens
        );
    }

    pub fn reveal_masked(&self) -> MaskedConfig {
        MaskedConfig {
            llm: MaskedLlm {
                api_key: self.llm.api_key.as_ref().map(|k| MaskedSecret {
                    len: k.len(),
                    sha256_8: sha256_8(k),
                    starts_with: k.chars().take(5).collect(),
                    ends_with: k
                        .chars()
                        .rev()
                        .take(4)
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect(),
                }),
                provider: self.llm.provider.clone(),
                base_url: self.llm.base_url.clone(),
                model_primary: self.llm.model_primary.clone(),
                model_fallbacks: self.llm.model_fallbacks.clone(),
                timeout_secs: self.llm.timeout_secs,
                max_tokens: self.llm.max_tokens,
                temperature: self.llm.temperature,
            },
            http: self.http.clone(),
            ingest: self.ingest.clone(),
            search: self.search.clone(),
            hybrid: self.hybrid.clone(),
        }
    }
}

// ===== helpers =====
fn normalize_secret(s: &str) -> String {
    let mut t = s.trim().trim_matches('\u{feff}').to_string();
    if (t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')) {
        t = t[1..t.len() - 1].to_string();
    }
    while t.ends_with('\r') || t.ends_with('\n') || t.ends_with(' ') || t.ends_with('\t') {
        t.pop();
    }
    t
}

fn validate_api_key(k: &str) -> anyhow::Result<()> {
    if k.len() < 20 {
        anyhow::bail!("LLM api_key too short ({} chars)", k.len());
    }
    if k.contains('\n') || k.contains('\r') {
        anyhow::bail!("LLM api_key contains newline");
    }
    Ok(())
}

fn sha256_8(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    let d = h.finalize();
    hex::encode(&d[..4])
}

// ===== masked view for /api/health/config =====
#[derive(serde::Serialize, Clone, Debug)]
pub struct MaskedConfig {
    pub llm: MaskedLlm,
    pub http: HttpSection,
    pub ingest: IngestSection,
    pub search: SearchSection,
    pub hybrid: HybridSection,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct MaskedLlm {
    pub api_key: Option<MaskedSecret>,
    pub provider: String,
    pub base_url: String,
    pub model_primary: String,
    pub model_fallbacks: Vec<String>,
    pub timeout_secs: u64,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct MaskedSecret {
    pub len: usize,
    pub sha256_8: String,
    pub starts_with: String,
    pub ends_with: String,
}

// WARN + default (строки)
fn get_env_or_warn(name: &str, default: impl Into<String>) -> String {
    match std::env::var(name) {
        Ok(v) => v.trim().to_string(),
        Err(_) => {
            tracing::warn!("ENV `{}` not set, using default", name);
            default.into()
        }
    }
}

fn get_env_num_or_warn<T>(name: &str, default: T) -> T
where
    T: std::str::FromStr + Copy,
{
    match std::env::var(name) {
        Ok(v) => match v.trim().parse::<T>() {
            Ok(x) => x,
            Err(_) => {
                tracing::warn!("ENV `{}` invalid value `{}`, using default", name, v);
                default
            }
        },
        Err(_) => {
            tracing::warn!("ENV `{}` not set, using default", name);
            default
        }
    }
}

/// СЕКРЕТЫ — если нет: **сразу ошибка**
fn require_secret(name: &str) -> AnyResult<String> {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => Ok(v.trim().to_string()),
        _ => {
            tracing::error!("❌ REQUIRED SECRET `{}` missing. Refusing to start.", name);
            Err(anyhow::anyhow!("Missing required secret `{}`", name))
        }
    }
}
