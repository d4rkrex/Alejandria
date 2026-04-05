use crate::error::IcmResult;

/// Trait for embedding text into vector representations.
///
/// Implementations provide model-specific embedding generation for semantic search.
/// All methods are synchronous (no async) following Alejandria's architecture.
pub trait Embedder: Send + Sync {
    /// Generate embedding vector for a single text input.
    ///
    /// # Arguments
    /// * `text` - The text to embed
    ///
    /// # Returns
    /// A vector of f32 values representing the embedding (dimensions depend on model)
    ///
    /// # Errors
    /// Returns IcmError::Embedding if the model fails to generate embeddings
    fn embed(&self, text: &str) -> IcmResult<Vec<f32>>;

    /// Generate embeddings for multiple texts in a batch.
    ///
    /// Batch processing is more efficient than calling embed() repeatedly.
    ///
    /// # Arguments
    /// * `texts` - Slice of text references to embed
    ///
    /// # Returns
    /// A vector of embeddings, one per input text
    ///
    /// # Errors
    /// Returns IcmError::Embedding if the model fails to generate embeddings
    fn embed_batch(&self, texts: &[&str]) -> IcmResult<Vec<Vec<f32>>>;

    /// Returns the dimensionality of embeddings produced by this model.
    ///
    /// # Examples
    /// - multilingual-e5-base: 768
    /// - multilingual-e5-small: 384
    /// - text-embedding-3-small (OpenAI): 1536
    fn dimensions(&self) -> usize;

    /// Returns the name/identifier of the embedding model being used.
    fn model_name(&self) -> &str;
}
