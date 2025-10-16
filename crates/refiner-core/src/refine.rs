// ─────────────────────────────────────────────────────────────────────────────
// /crates/refiner-core/src/refine.rs
// ─────────────────────────────────────────────────────────────────────────────

use crate::types::{Composite, Document, RefineOptions, Unit};
use llm_traits::{ChatMessage, ChatRequest, LlmClient};
use prompts::PromptBank;
use refiner_cache as cache;
use tracing::{debug, info};
use serde::de::DeserializeOwned;
use regex::Regex;

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

    /// Очень простая «ленивая» попытка прочитать JSON от модели:
    /// - берём подстроку между первым '{' и последним '}'
    /// - убираем висячие запятые перед '}' или ']'
    /// - парсим через serde_json
    fn parse_json_relaxed<T: DeserializeOwned>(raw: &str) -> anyhow::Result<T> {
        let s = Self::strip_code_fences(raw);
        let start = s.find('{').ok_or_else(|| anyhow::anyhow!("no opening brace in model output"))?;
        let end = s.rfind('}').ok_or_else(|| anyhow::anyhow!("no closing brace in model output"))?;
        let core = &s[start..=end];

        // Удаляем висячие запятые: ", }" → " }", ", ]" → " ]"
        let re_trailing = Regex::new(r",(\s*[\}\]])").unwrap();
        let cleaned = re_trailing.replace_all(core, "$1");

        let v = serde_json::from_str::<T>(&cleaned)
            .map_err(|e| anyhow::anyhow!("relaxed json parse failed: {e}. snippet: {}", &cleaned.chars().take(200).collect::<String>()))?;
        Ok(v)
    }

    /// Фоллбек: вытащить title/summary «грубой силой» из произвольного текста модели.
    /// Возвращает Composite с пустыми topics, чтобы пайплайн не падал.
    fn parse_composite_heuristic(raw: &str) -> Composite {
        let s = Self::strip_code_fences(raw);

        // Попытаемся найти "title": "..." и "summary": "..."
        let re_title = Regex::new(r#"(?is)"title"\s*:\s*"([^"]*)"#).unwrap();
        let re_summary = Regex::new(r#"(?is)"summary"\s*:\s*"([^"]*)"#).unwrap();

        let mut title = re_title
            .captures(s)
            .and_then(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
            .unwrap_or_else(|| "Auto title".to_string());

        let mut summary = re_summary
            .captures(s)
            .and_then(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
            .unwrap_or_else(|| "Auto summary".to_string());

        // Если строки обрезаны (часто заканчиваются на незакрытую кавычку до перевода строки),
        // чуть подчистим: оборвём по последней законченной фразе/точке.
        let cut_to_sentence = |t: &mut String| {
            if let Some(pos) = t.rfind(['.', '!', '?']) {
                if pos + 1 < t.len() {
                    t.truncate(pos + 1);
                }
            }
            // Сменим «умные» кавычки на обычные
            *t = t.replace(['“','”','„','«','»'], "\"");
        };
        cut_to_sentence(&mut title);
        cut_to_sentence(&mut summary);

        Composite { title, summary, topics: Vec::new() }
    }

    /// Рефайн текста юнита → записываем в unit.refined
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
                debug!("Cache hit (refine) for unit {} ({})", i, u.id);
                let mut nu = u.clone();
                nu.refined = Some(hit);
                out.push(nu);
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
            let refined = resp.choices.first().map(|c| c.message.content.clone()).unwrap_or_default();
            cache::write(self.cache_dir, &key, &refined).await?;
            let mut nu = u.clone();
            nu.refined = Some(refined);
            out.push(nu);
        }
        Ok(out)
    }

    /// Пер-юнитные summary/keyphrases/entities/topics (RU/EN)
    pub async fn enrich_units(
        &self,
        units: &mut [Unit],
        opts: &RefineOptions,
    ) -> anyhow::Result<()> {
        // Шаблоны
        let sys_sum = self.prompts.get("unit_summary/system");
        let user_sum_short = self.prompts.get("unit_summary/short_user");
        let user_sum_long  = self.prompts.get("unit_summary/long_user");

        let sys_keys = self.prompts.get("unit_keyphrases/system");
        let user_keys = self.prompts.get("unit_keyphrases/user");

        let sys_ents = self.prompts.get("unit_entities_topics/system");
        let user_ents = self.prompts.get("unit_entities_topics/user");

        anyhow::ensure!(!sys_sum.is_empty() && !user_sum_short.is_empty() && !user_sum_long.is_empty(),
            "missing prompts: unit_summary/*");
        anyhow::ensure!(!sys_keys.is_empty() && !user_keys.is_empty(),
            "missing prompts: unit_keyphrases/*");
        anyhow::ensure!(!sys_ents.is_empty() && !user_ents.is_empty(),
            "missing prompts: unit_entities_topics/*");

        // Хелпер: запрос → строка
        async fn ask<C: LlmClient>(llm: &C, model: &str, sys: &str, user: String, temp: f32, max_tokens: u32)
            -> anyhow::Result<String>
        {
            let req = ChatRequest {
                model: model.to_string(),
                messages: vec![
                    ChatMessage { role: "system".into(), content: sys.to_string() },
                    ChatMessage { role: "user".into(), content: user },
                ],
                temperature: Some(temp),
                max_tokens: Some(max_tokens as u32),
            };
            let resp = llm.chat(req).await?;
            Ok(resp.choices.first().map(|c| c.message.content.clone()).unwrap_or_default())
        }

        for (i, u) in units.iter_mut().enumerate() {
            let basis = u.refined.as_ref().map(|s| s.as_str()).unwrap_or(&u.text);

            // short summary
            {
                let user = user_sum_short.replace("{{TEXT}}", basis);
                let key = cache::key_for(format!("sum_short:{}:{}:{}", self.model, opts.temperature, &user).as_bytes());
                if let Some(hit) = cache::read(self.cache_dir, &key).await? {
                    debug!("Cache hit (sum_short) for unit {} ({})", i, u.id);
                    u.summary_short = Some(hit);
                } else {
                    info!("LLM call → unit {} summary_short", u.id);
                    let s = ask(self.llm, self.model, &sys_sum, user, 0.1, 96).await?;
                    cache::write(self.cache_dir, &key, &s).await?;
                    u.summary_short = Some(s);
                }
            }

            // long summary
            {
                let user = user_sum_long.replace("{{TEXT}}", basis);
                let key = cache::key_for(format!("sum_long:{}:{}:{}", self.model, opts.temperature, &user).as_bytes());
                if let Some(hit) = cache::read(self.cache_dir, &key).await? {
                    debug!("Cache hit (sum_long) for unit {} ({})", i, u.id);
                    u.summary_long = Some(hit);
                } else {
                    info!("LLM call → unit {} summary_long", u.id);
                    let s = ask(self.llm, self.model, &sys_sum, user, 0.1, 256).await?;
                    cache::write(self.cache_dir, &key, &s).await?;
                    u.summary_long = Some(s);
                }
            }

            // keyphrases
            {
                let user = user_keys.replace("{{TEXT}}", basis);
                let key = cache::key_for(format!("keyphrases:{}:{}:{}", self.model, opts.temperature, &user).as_bytes());
                if let Some(hit) = cache::read(self.cache_dir, &key).await? {
                    debug!("Cache hit (keyphrases) for unit {} ({})", i, u.id);
                    u.keyphrases = serde_json::from_str::<Vec<String>>(&hit).unwrap_or_else(|_| vec![]);
                } else {
                    info!("LLM call → unit {} keyphrases", u.id);
                    let s = ask(self.llm, self.model, &sys_keys, user, 0.0, 128).await?;
                    let json = Self::strip_code_fences(&s);
                    let v: Vec<String> = serde_json::from_str(json).unwrap_or_else(|_| vec![]);
                    cache::write(self.cache_dir, &key, json).await?;
                    u.keyphrases = v;
                }
            }

            // entities + topics ru/en
            {
                let user = user_ents.replace("{{TEXT}}", basis);
                let key = cache::key_for(format!("ents_topics:{}:{}:{}", self.model, opts.temperature, &user).as_bytes());
                if let Some(hit) = cache::read(self.cache_dir, &key).await? {
                    debug!("Cache hit (entities/topics) for unit {} ({})", i, u.id);
                    #[derive(serde::Deserialize)]
                    struct EntOut { entities: Vec<String>, topics_ru: Vec<String>, topics_en: Vec<String> }
                    let parsed: EntOut = serde_json::from_str(&hit).unwrap_or(EntOut{entities:vec![],topics_ru:vec![],topics_en:vec![]});
                    u.entities = parsed.entities;
                    u.topics_ru = parsed.topics_ru;
                    u.topics_en = parsed.topics_en;
                } else {
                    info!("LLM call → unit {} entities/topics", u.id);
                    let s = ask(self.llm, self.model, &sys_ents, user, 0.0, 256).await?;
                    let json = Self::strip_code_fences(&s);
                    #[derive(serde::Deserialize)]
                    struct EntOut { entities: Vec<String>, topics_ru: Vec<String>, topics_en: Vec<String> }
                    let parsed: EntOut = serde_json::from_str(json).unwrap_or(EntOut{entities:vec![],topics_ru:vec![],topics_en:vec![]});
                    cache::write(self.cache_dir, &key, json).await?;
                    u.entities = parsed.entities;
                    u.topics_ru = parsed.topics_ru;
                    u.topics_en = parsed.topics_en;
                }
            }
        }
        Ok(())
    }

    /// Верхнеуровневая сводка всего документа
    pub async fn build_composite(
        &self,
        units: &[Unit],
        opts: &RefineOptions,
    ) -> anyhow::Result<Composite> {
        info!("LLM call → build composite ({} units)", units.len());
        // В композит берём «базис» — refined если есть, иначе original.
        let joined = units.iter()
            .map(|u| u.refined.as_ref().map(|s| s.as_str()).unwrap_or(u.text.as_str()))
            .collect::<Vec<_>>()
            .join("\n\n");

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
            // сначала строгий парсер, затем relaxed, затем эвристика
            if let Ok(comp) = serde_json::from_str::<Composite>(&hit) {
                return Ok(comp);
            }
            if let Ok(comp) = Self::parse_json_relaxed::<Composite>(&hit) {
                return Ok(comp);
            }
            let comp = Self::parse_composite_heuristic(&hit);
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

        // 1) строгий 2) relaxed 3) эвристика
        let comp = serde_json::from_str::<Composite>(&raw)
            .or_else(|_| Self::parse_json_relaxed::<Composite>(&raw))
            .unwrap_or_else(|_| Self::parse_composite_heuristic(&raw));

        // Кладём в кэш уже «почищенную» версию
        let cleaned = serde_json::to_string(&comp)?;
        cache::write(self.cache_dir, &key, &cleaned).await?;
        Ok(comp)
    }

    /// Удобный адаптер под старый формат (document.summary)
    pub async fn build_document(&self, units: &[Unit], opts: &RefineOptions) -> anyhow::Result<Document> {
        let comp = self.build_composite(units, opts).await?;
        Ok(Document { summary: comp.summary })
    }
}
