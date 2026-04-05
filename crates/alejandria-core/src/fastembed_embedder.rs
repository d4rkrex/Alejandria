#[cfg(feature = "embeddings")]
use std::path::PathBuf;
#[cfg(feature = "embeddings")]
use std::sync::{Mutex, OnceLock};

#[cfg(feature = "embeddings")]
use directories::ProjectDirs;
#[cfg(feature = "embeddings")]
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

use crate::embedder::Embedder;
use crate::error::{IcmError, IcmResult};

/// Cache directory for embedding models (multi-OS via `directories`).
/// - macOS: ~/Library/Caches/dev.alejandria.alejandria/models/
/// - Linux: ~/.cache/alejandria/models/
/// - Windows: C:\Users\<user>\AppData\Local\alejandria\alejandria\cache\models\
#[cfg(feature = "embeddings")]
fn cache_dir() -> PathBuf {
    ProjectDirs::from("dev", "alejandria", "alejandria")
        .map(|dirs| dirs.cache_dir().join("models"))
        .unwrap_or_else(|| {
            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home)
                .join(".cache")
                .join("alejandria")
                .join("models")
        })
}

/// Fastembed-based embedder using multilingual-e5-base (768 dimensions).
///
/// This implementation uses lazy loading with a thread-safe singleton pattern
/// to avoid loading the model multiple times. The model is downloaded on first
/// use and cached for subsequent calls.
#[cfg(feature = "embeddings")]
pub struct FastembedEmbedder {
    model: OnceLock<TextEmbedding>,
    init_lock: Mutex<()>,
    model_name: String,
    dims: usize,
}

/// Default model: multilingual-e5-base (768d, supports 100+ languages)
#[cfg(feature = "embeddings")]
const DEFAULT_MODEL: &str = "intfloat/multilingual-e5-base";

/// Resolve a model string to (EmbeddingModel, dimensions).
#[cfg(feature = "embeddings")]
fn resolve_model(name: &str) -> IcmResult<(EmbeddingModel, usize)> {
    let model: EmbeddingModel = name.parse().map_err(|e: String| IcmError::Embedding(e))?;
    let dims = model_dimensions(&model);
    Ok((model, dims))
}

/// Known dimensions for fastembed models.
#[cfg(feature = "embeddings")]
fn model_dimensions(model: &EmbeddingModel) -> usize {
    match model {
        EmbeddingModel::AllMiniLML6V2
        | EmbeddingModel::AllMiniLML6V2Q
        | EmbeddingModel::AllMiniLML12V2
        | EmbeddingModel::AllMiniLML12V2Q
        | EmbeddingModel::BGESmallENV15
        | EmbeddingModel::BGESmallENV15Q
        | EmbeddingModel::MultilingualE5Small
        | EmbeddingModel::ParaphraseMLMiniLML12V2
        | EmbeddingModel::ParaphraseMLMiniLML12V2Q => 384,

        EmbeddingModel::BGEBaseENV15
        | EmbeddingModel::BGEBaseENV15Q
        | EmbeddingModel::MultilingualE5Base
        | EmbeddingModel::ParaphraseMLMpnetBaseV2
        | EmbeddingModel::BGESmallZHV15
        | EmbeddingModel::GTEBaseENV15
        | EmbeddingModel::GTEBaseENV15Q
        | EmbeddingModel::JinaEmbeddingsV2BaseCode => 768,

        EmbeddingModel::BGELargeENV15
        | EmbeddingModel::BGELargeENV15Q
        | EmbeddingModel::MultilingualE5Large
        | EmbeddingModel::MxbaiEmbedLargeV1
        | EmbeddingModel::MxbaiEmbedLargeV1Q
        | EmbeddingModel::BGELargeZHV15
        | EmbeddingModel::GTELargeENV15
        | EmbeddingModel::GTELargeENV15Q
        | EmbeddingModel::ModernBertEmbedLarge => 1024,

        EmbeddingModel::NomicEmbedTextV1
        | EmbeddingModel::NomicEmbedTextV15
        | EmbeddingModel::NomicEmbedTextV15Q => 768,

        EmbeddingModel::ClipVitB32 => 512,
    }
}

#[cfg(feature = "embeddings")]
impl FastembedEmbedder {
    /// Create with default model (multilingual-e5-base, 768 dimensions).
    pub fn new() -> Self {
        Self::with_model(DEFAULT_MODEL)
    }

    /// Create with a specific model by name (e.g. "intfloat/multilingual-e5-base").
    ///
    /// # Arguments
    /// * `model_name` - The fastembed model identifier
    ///
    /// # Panics
    /// Does not panic - if model name is invalid, will return error on first embed() call
    pub fn with_model(model_name: &str) -> Self {
        let dims = resolve_model(model_name).map(|(_, d)| d).unwrap_or(768);
        Self {
            model: OnceLock::new(),
            init_lock: Mutex::new(()),
            model_name: model_name.to_string(),
            dims,
        }
    }

    /// Lazy-load the embedding model (thread-safe singleton pattern).
    fn get_model(&self) -> IcmResult<&TextEmbedding> {
        // Fast path: model already loaded
        if let Some(m) = self.model.get() {
            return Ok(m);
        }

        // Slow path: need to load model (lock to prevent concurrent loads)
        let _guard = self.init_lock.lock().unwrap();

        // Double-check after acquiring lock
        if let Some(m) = self.model.get() {
            return Ok(m);
        }

        // Load model
        let (emb_model, _) = resolve_model(&self.model_name)?;
        let cache = cache_dir();
        let model = TextEmbedding::try_new(
            InitOptions::new(emb_model)
                .with_show_download_progress(true)
                .with_cache_dir(cache),
        )
        .map_err(|e| IcmError::Embedding(format!("failed to init model: {e}")))?;

        let _ = self.model.set(model);
        Ok(self.model.get().unwrap())
    }
}

#[cfg(feature = "embeddings")]
impl Default for FastembedEmbedder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "embeddings")]
impl Embedder for FastembedEmbedder {
    fn embed(&self, text: &str) -> IcmResult<Vec<f32>> {
        let model = self.get_model()?;
        let results = model
            .embed(vec![text], None)
            .map_err(|e| IcmError::Embedding(e.to_string()))?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| IcmError::Embedding("empty embedding result".into()))
    }

    fn embed_batch(&self, texts: &[&str]) -> IcmResult<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let model = self.get_model()?;
        model
            .embed(texts.to_vec(), None)
            .map_err(|e| IcmError::Embedding(e.to_string()))
    }

    fn dimensions(&self) -> usize {
        self.dims
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

// Stub implementation when embeddings feature is disabled
#[cfg(not(feature = "embeddings"))]
pub struct FastembedEmbedder;

#[cfg(not(feature = "embeddings"))]
impl FastembedEmbedder {
    pub fn new() -> Self {
        Self
    }

    pub fn with_model(_model_name: &str) -> Self {
        Self
    }
}

#[cfg(not(feature = "embeddings"))]
impl Default for FastembedEmbedder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "embeddings"))]
impl Embedder for FastembedEmbedder {
    fn embed(&self, _text: &str) -> IcmResult<Vec<f32>> {
        Err(IcmError::Embedding(
            "embeddings feature is disabled - enable with --features embeddings".into(),
        ))
    }

    fn embed_batch(&self, _texts: &[&str]) -> IcmResult<Vec<Vec<f32>>> {
        Err(IcmError::Embedding(
            "embeddings feature is disabled - enable with --features embeddings".into(),
        ))
    }

    fn dimensions(&self) -> usize {
        768
    }

    fn model_name(&self) -> &str {
        "disabled"
    }
}
