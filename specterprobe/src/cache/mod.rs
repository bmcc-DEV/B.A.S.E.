/// Cache incremental para o pipeline.
/// Evita reprocessar firmwares que já foram analisados.
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Chave de cache baseada no hash SHA-256 dos dados de entrada + configuração
#[derive(Debug, Clone)]
pub struct CacheKey {
    pub data_hash: String,
    pub pipeline_stage: String,
}

impl CacheKey {
    pub fn new(data: &[u8], stage: &str) -> Self {
        let hash = hex::encode(Sha256::digest(data));
        Self {
            data_hash: hash,
            pipeline_stage: stage.to_string(),
        }
    }

    pub fn file_name(&self) -> String {
        format!("{}_{}", self.pipeline_stage, self.data_hash)
    }
}

/// Cache baseado em arquivos no diretório de cache
pub struct PipelineCache {
    cache_dir: PathBuf,
}

impl PipelineCache {
    /// Cria cache no diretório `output_dir/.cache/`.
    /// Se `output_dir` for None, cache é desabilitado.
    pub fn new(output_dir: Option<&Path>) -> Self {
        let cache_dir = output_dir
            .map(|d| d.join(".cache"))
            .unwrap_or_else(|| PathBuf::from("/tmp/specterprobe_cache"));
        std::fs::create_dir_all(&cache_dir).ok();
        Self { cache_dir }
    }

    /// Tenta carregar do cache. Se não existir, computa com `compute()` e salva.
    pub fn get_or_compute<T: serde::Serialize + serde::de::DeserializeOwned>(
        &self,
        key: &CacheKey,
        compute: impl FnOnce() -> T,
    ) -> T {
        let path = self.cache_path(key);

        if path.exists() {
            tracing::info!("Cache HIT: {}", key.file_name());
            match std::fs::File::open(&path) {
                Ok(file) => match serde_json::from_reader(file) {
                    Ok(val) => return val,
                    Err(e) => tracing::warn!("Cache read error: {e}, recomputing"),
                },
                Err(e) => tracing::warn!("Cache open error: {e}, recomputing"),
            }
        }

        tracing::info!("Cache MISS: {}", key.file_name());
        let result = compute();

        if let Err(e) = self.save(&path, &result) {
            tracing::warn!("Cache write error: {e}");
        }

        result
    }

    fn cache_path(&self, key: &CacheKey) -> PathBuf {
        self.cache_dir.join(format!("{}.json", key.file_name()))
    }

    fn save<T: serde::Serialize>(&self, path: &Path, value: &T) -> Result<(), std::io::Error> {
        let file = std::fs::File::create(path)?;
        serde_json::to_writer(file, value)?;
        Ok(())
    }

    /// Limpa cache antigo (arquivos com mais de `max_age_days` dias)
    pub fn clean_old(&self, max_age_days: u64) {
        let max_age = std::time::Duration::from_secs(max_age_days * 86400);
        let now = std::time::SystemTime::now();

        if let Ok(entries) = std::fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if now.duration_since(modified).map_or(false, |age| age > max_age) {
                            std::fs::remove_file(entry.path()).ok();
                        }
                    }
                }
            }
        }
    }

    /// Remove todas as entradas de cache para um estágio específico
    pub fn invalidate_stage(&self, stage: &str) {
        if let Ok(entries) = std::fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                if name.to_string_lossy().starts_with(stage) {
                    std::fs::remove_file(entry.path()).ok();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cache_hit() {
        let dir = tempdir().unwrap();
        let cache = PipelineCache::new(Some(dir.path()));

        let key = CacheKey::new(b"hello", "test");
        let result: String = cache.get_or_compute(&key, || "computed".to_string());
        assert_eq!(result, "computed");

        // Second call should return cached value (function not called)
        let result2: String = cache.get_or_compute(&key, || panic!("should not recompute"));
        assert_eq!(result2, "computed");
    }

    #[test]
    fn test_cache_key_unique() {
        let k1 = CacheKey::new(b"data1", "lift");
        let k2 = CacheKey::new(b"data2", "lift");
        assert_ne!(k1.file_name(), k2.file_name(), "Different data should produce different keys");

        let k3 = CacheKey::new(b"data1", "mmio");
        assert_ne!(k1.file_name(), k3.file_name(), "Different stages should produce different keys");
    }

    #[test]
    fn test_cache_invalidate() {
        let dir = tempdir().unwrap();
        let cache = PipelineCache::new(Some(dir.path()));

        let key = CacheKey::new(b"test", "lift");
        cache.get_or_compute(&key, || 42u64);

        let cache_file = dir.path().join(".cache").join(format!("{}.json", key.file_name()));
        assert!(cache_file.exists(), "Cache file should exist");

        cache.invalidate_stage("lift");
        assert!(!cache_file.exists(), "Cache file should be removed after invalidation");
    }
}
