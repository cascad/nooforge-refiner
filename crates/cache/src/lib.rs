use std::path::Path;
use sha2::{Digest, Sha256};
use camino::Utf8PathBuf;


pub fn key_for(bytes: &[u8]) -> String {
let mut hasher = Sha256::new();
hasher.update(bytes);
hex::encode(hasher.finalize())
}


pub async fn write(cache_dir: impl AsRef<Path>, key: &str, data: &str) -> anyhow::Result<()> {
let dir: Utf8PathBuf = cache_dir.as_ref().to_path_buf().try_into()?;
tokio::fs::create_dir_all(&dir).await?;
let path = dir.join(format!("{}.json", key));
tokio::fs::write(path, data).await?;
Ok(())
}


pub async fn read(cache_dir: impl AsRef<Path>, key: &str) -> anyhow::Result<Option<String>> {
let dir: Utf8PathBuf = cache_dir.as_ref().to_path_buf().try_into()?;
let path = dir.join(format!("{}.json", key));
match tokio::fs::read_to_string(path).await {
Ok(s) => Ok(Some(s)),
Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
Err(e) => Err(e.into()),
}
}