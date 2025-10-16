use crate::{
    refine::Refiners,
    segmenter::split_sentences,
    types::{Document, PipelineConfig, PipelineOutput, RefineOptions, SegMode, Unit},
};
use llm_traits::LlmClient;
use tracing::{debug, info};

pub async fn run<C: LlmClient>(
    llm: &C,
    cfg: &PipelineConfig,
    input_text: &str,
) -> anyhow::Result<PipelineOutput> {
    info!("🚀 Starting pipeline (model = {}, mode = {:?})", cfg.model, cfg.seg_mode);

    // 1) Segментация
    info!("🔹 Step 1: segmentation");
    let mut units: Vec<Unit> = if cfg.sentence_units {
        split_sentences(input_text)
    } else {
        vec![Unit {
            id: "u00000".into(),
            text: input_text.to_string(),
            start_char: 0,
            end_char: input_text.len(),
            refined: None,
            summary_short: None,
            summary_long: None,
            keyphrases: Vec::new(),
            entities: Vec::new(),
            topics_ru: Vec::new(),
            topics_en: Vec::new(),
        }]
    };
    info!("🔸 Segmentation complete: {} units", units.len());

    // 2) Refinement + per-unit enrichment + document summary
    let opts = RefineOptions {
        bilingual_topics: true,
        temperature: 0.1,
        max_tokens: Some(256),
    };
    let bank = prompts::PromptBank::autodetect();
    let r = Refiners {
        llm,
        model: &cfg.model,
        cache_dir: &cfg.cache_dir,
        prompts: &bank,
    };

    let mut document: Option<Document> = None;
    match cfg.seg_mode {
        SegMode::SentencesOnly => {
            info!("🟢 SegMode = SentencesOnly, skipping LLM refinement/enrichment");
        }
        SegMode::LlmFirst | SegMode::Auto => {
            info!("🔹 Step 2: refining {} units...", units.len());
            units = r.refine_units(&units, &opts).await?;
            debug!("Refinement complete. Enriching units...");
            r.enrich_units(&mut units, &opts).await?;
            debug!("Enrichment complete. Building document summary...");
            document = Some(r.build_document(&units, &opts).await?);
        }
    }

    info!("✅ Pipeline finished: {} units", units.len());
    Ok(PipelineOutput { units, document })
}
