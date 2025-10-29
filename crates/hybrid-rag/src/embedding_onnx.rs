// src/embedding_onnx.rs
use ort::{Session, Value, Environment};

pub struct ONNXEmbedder {
    session: Session,
}

impl ONNXEmbedder {
    pub fn new(model_path: &str) -> Result<Self> {
        let environment = Environment::builder()
            .with_name("embedder")
            .build()?;
            
        let session = Session::builder(&environment)?
            .with_optimization_level(ort::GraphOptimizationLevel::All)?
            .with_intra_threads(4)?
            .commit_from_file(model_path)?;
            
        Ok(Self { session })
    }
}

#[async_trait]
impl Embedder for ONNXEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Токенизация
        let tokens = self.tokenize(text);
        let input_ids = Value::from_array(([1, tokens.len()], &tokens))?;
        
        // Инференс
        let outputs = self.session.run(vec![input_ids])?;
        let embeddings: &[f32] = outputs[0].try_extract_tensor()?.view().as_slice()?;
        
        Ok(embeddings.to_vec())
    }
}