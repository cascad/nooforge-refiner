use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "prompts/"]
struct Embedded;

/// PromptBank загружает промпты из переменной окружения `NOOFORGE_PROMPTS_DIR`,
/// если она задана, иначе — из встроенных ресурсов.
pub struct PromptBank {
    root: Option<std::path::PathBuf>,
}

impl PromptBank {
    pub fn autodetect() -> Self {
        let root = std::env::var_os("NOOFORGE_PROMPTS_DIR").map(std::path::PathBuf::from);
        Self { root }
    }

    pub fn get(&self, name: &str) -> String {
        // имя вида "refine/system" → файл refine/system.txt
        let rel = format!("{}.txt", name);
        if let Some(root) = &self.root {
            let p = root.join(&rel);
            if let Ok(text) = std::fs::read_to_string(&p) {
                return text;
            }
        }
        if let Some(data) = Embedded::get(&rel) {
            return String::from_utf8_lossy(&data.data).into_owned();
        }
        String::new()
    }
}
