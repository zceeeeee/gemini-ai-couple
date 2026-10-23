pub mod engine;
pub mod math;

pub use engine::{DeterministicEmbeddingEngine, EmbeddingEngine, SharedEmbeddingEngine};
pub use math::{cosine_similarity, dot_product, l2_norm, normalize_in_place};
