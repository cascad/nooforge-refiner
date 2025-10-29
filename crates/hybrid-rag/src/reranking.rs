use anyhow::Result;
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use tokenizers::Tokenizer;

pub struct CrossEncoderReranker {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl CrossEncoderReranker {
    pub fn new(model_name: &str) -> Result<Self> {
        let device = Device::Cpu;
        
        let tokenizer = Tokenizer::from_pretrained(model_name, None)?;
        
        let config = Config::tiny();
        let vb = VarBuilder::from_pretrained(model_name, &device)?;
        let model = BertModel::load(vb, &config)?;
        
        Ok(Self {
            model,
            tokenizer,
            device,
        })
    }

    pub async fn rerank(&self, query: &str, documents: &[&str]) -> Result<Vec<f32>> {
        let mut scores = Vec::new();

        for &doc in documents {
            // Формируем пару (query, document) для cross-encoder
            let input_text = format!("{} [SEP] {}", query, doc);
            
            let encoding = self.tokenizer.encode(input_text, true)?;
            let tokens = encoding.get_ids();
            
            let tokens_tensor = Tensor::new(tokens, &self.device)?.unsqueeze(0)?;
            let token_type_ids = Tensor::zeros((1, tokens.len() as u64), candle_core::DType::U32, &self.device)?;
            
            let output = self.model.forward(&tokens_tensor, &token_type_ids)?;
            
            // Берем скор из [CLS] токена
            let cls_embedding = output.i((0, 0))?;
            let score = cls_embedding.to_vec1::<f32>()?[0]; // Упрощенный подход
            
            scores.push(score);
        }

        // Нормализуем scores от 0 до 1
        if let (Some(&min), Some(&max)) = (scores.iter().min_by(|a, b| a.partial_cmp(b).unwrap()), 
                                         scores.iter().max_by(|a, b| a.partial_cmp(b).unwrap())) {
            if max != min {
                for score in &mut scores {
                    *score = (*score - min) / (max - min);
                }
            }
        }

        Ok(scores)
    }
}