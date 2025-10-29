use anyhow::{Result, anyhow};
use ort::{Session, Value, Environment, GraphOptimizationLevel};
use tokenizers::Tokenizer;

pub struct ONNXCrossEncoder {
    session: Session,
    tokenizer: Tokenizer,
}

impl ONNXCrossEncoder {
    pub fn new(model_path: &str) -> Result<Self> {
        let environment = Environment::builder()
            .with_name("cross_encoder")
            .build()?;
        
        let session = Session::builder(&environment)?
            .with_optimization_level(GraphOptimizationLevel::All)?
            .commit_from_file(model_path)?;
        
        // Для cross-encoder используем токенизатор BERT
        let tokenizer = Tokenizer::from_pretrained("bert-base-uncased", None)
            .map_err(|e| anyhow!("Failed to load tokenizer: {}", e))?;
        
        Ok(Self { session, tokenizer })
    }
    
    pub fn rerank(&self, query: &str, documents: &[String]) -> Result<Vec<f32>> {
        let mut scores = Vec::new();
        
        for doc in documents {
            // Форматируем вход для cross-encoder: [CLS] query [SEP] document [SEP]
            let input_text = format!("{} [SEP] {}", query, doc);
            
            let encoding = self.tokenizer.encode(input_text, true)
                .map_err(|e| anyhow!("Tokenization failed: {}", e))?;
            
            let input_ids = encoding.get_ids();
            let attention_mask = encoding.get_attention_mask();
            let token_type_ids = encoding.get_type_ids();
            
            // Подготовка входных данных
            let input_ids_tensor = Value::from_array(
                (1, input_ids.len()),
                &input_ids.iter().map(|&x| x as i64).collect::<Vec<i64>>()
            )?;
            
            let attention_mask_tensor = Value::from_array(
                (1, input_ids.len()),
                &attention_mask.iter().map(|&x| x as i64).collect::<Vec<i64>>()
            )?;
            
            // Инференс
            let outputs = self.session.run(vec![
                input_ids_tensor,
                attention_mask_tensor
            ])?;
            
            // Получение скора релевантности
            let logits = &outputs[0];
            let logits_slice: &[f32] = logits.try_extract_tensor()?.view().as_slice()?;
            
            // Берем скор для позитивного класса (обычно индекс 1)
            let score = if logits_slice.len() > 1 {
                logits_slice[1] // Бинарная классификация
            } else {
                logits_slice[0] // Регрессия
            };
            
            scores.push(score);
        }
        
        Ok(scores)
    }
}