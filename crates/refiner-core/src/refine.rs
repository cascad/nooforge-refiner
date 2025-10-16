use crate::types::{Composite, RefineOptions, Unit};
use llm_traits::{ChatMessage, ChatRequest, LlmClient};
use prompts::PromptBank;
use refiner_cache as cache;
use tracing::{debug, info};

pub struct Refiners<'a, C: LlmClient> {
    pub llm: &'a C,
    pub model: &'a str,
    pub cache_dir: &'a str,
    pub prompts: &'a PromptBank,
}

impl<'a, C: LlmClient> Refiners<'a, C> {
    #[inline]
    fn strip_code_fences(s: &str) -> &str {
        let s = s.trim();
        let s = s.strip_prefix("```json").unwrap_or(s);
        let s = s.strip_prefix("```").unwrap_or(s);
        let s = s.strip_suffix("```").unwrap_or(s);
        s.trim()
    }

    pub async fn refine_units(
        &self,
        units: &[Unit],
        opts: &RefineOptions,
    ) -> anyhow::Result<Vec<Unit>> {
        let sys = self.prompts.get("refine/system");
        let user_tpl = self.prompts.get("refine/user");
        anyhow::ensure!(!sys.is_empty(), "missing prompt: refine/system");
        anyhow::ensure!(!user_tpl.is_empty(), "missing prompt: refine/user");

        let mut out = Vec::with_capacity(units.len());
        for (i, u) in units.iter().enumerate() {
            let prompt = user_tpl.replace("{{TEXT}}", &u.text);
            let key_str = format!("refine:{}:{}:{}", self.model, opts.temperature, prompt);
            let key = cache::key_for(key_str.as_bytes());

            if let Some(hit) = cache::read(self.cache_dir, &key).await? {
                debug!("Cache hit for unit {} ({})", i, u.id);
                out.push(Unit { text: hit, ..u.clone() });
                continue;
            }

            info!("LLM call → refine unit {} ({} chars)", u.id, u.text.len());
            let req = ChatRequest {
                model: self.model.to_string(),
                messages: vec![
                    ChatMessage { role: "system".into(), content: sys.clone() },
                    ChatMessage { role: "user".into(), content: prompt },
                ],
                temperature: Some(opts.temperature),
                max_tokens: opts.max_tokens,
            };
            let resp = self.llm.chat(req).await?;
            let text = resp.choices.first().map(|c| c.message.content.clone()).unwrap_or_default();
            cache::write(self.cache_dir, &key, &text).await?;
            out.push(Unit { text, ..u.clone() });
        }
        Ok(out)
    }

    pub async fn build_composite(
        &self,
        units: &[Unit],
        opts: &RefineOptions,
    ) -> anyhow::Result<Composite> {
        info!("LLM call → build composite ({} units)", units.len());
        let joined = units.iter().map(|u| u.text.as_str()).collect::<Vec<_>>().join("\n\n");
        let sys = self.prompts.get("composite/system");
        let user_tpl = self.prompts.get("composite/user");
        anyhow::ensure!(!sys.is_empty(), "missing prompt: composite/system");
        anyhow::ensure!(!user_tpl.is_empty(), "missing prompt: composite/user");

        let prompt = user_tpl
            .replace("{{BILINGUAL}}", if opts.bilingual_topics { "bilingual (ru,en)" } else { "original language" })
            .replace("{{TEXT}}", &joined);

        let key_str = format!("composite:{}:{}:{}", self.model, opts.temperature, prompt);
        let key = cache::key_for(key_str.as_bytes());

        if let Some(hit) = cache::read(self.cache_dir, &key).await? {
            debug!("Cache hit for composite");
            let comp: Composite = serde_json::from_str(&hit)?;
            return Ok(comp);
        }

        let req = ChatRequest {
            model: self.model.to_string(),
            messages: vec![
                ChatMessage { role: "system".into(), content: sys },
                ChatMessage { role: "user".into(), content: prompt },
            ],
            temperature: Some(opts.temperature),
            max_tokens: Some(512),
        };
        let resp = self.llm.chat(req).await?;
        let raw = resp.choices.first().map(|c| c.message.content.clone()).unwrap_or_else(|| "{}".to_string());
        let json = Self::strip_code_fences(&raw);
        let comp: Composite = serde_json::from_str(json)?;
        cache::write(self.cache_dir, &key, json).await?;
        Ok(comp)
    }
}
