//! 嵌入引擎接口与实现

use super::math::normalize_in_place;
use crate::error::AppResult;

/// 向量嵌入通用引擎 Trait
pub trait EmbeddingEngine: Send + Sync {
    /// 对单一文本生成稠密特征向量
    fn embed(&self, text: &str) -> AppResult<Vec<f32>>;

    /// 批量文本嵌入
    fn embed_batch(&self, texts: &[String]) -> AppResult<Vec<Vec<f32>>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    /// 特征向量维度
    fn dimension(&self) -> usize;

    /// 引擎名称标识
    fn name(&self) -> &'static str;
}

/// 确定性轻量级语义特征引擎 (纯 Rust 实现，用于离线/单元测试与无模型环境)
///
/// 基于多尺度 N-Gram (Unigram, Bigram, Trigram) 与 FNV-1a 独立盐值特征哈希映射，
/// 使得字词重叠度、短语匹配度与余弦相似度呈现强正相关。
pub struct DeterministicEmbeddingEngine {
    dim: usize,
}

impl DeterministicEmbeddingEngine {
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }
}

impl Default for DeterministicEmbeddingEngine {
    fn default() -> Self {
        Self::new(512) // 默认 512 维，提供更低碰撞率
    }
}

fn fnv1a_hash(data: &[u8], seed: u64) -> u64 {
    let mut hash = seed;
    for &b in data {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

impl EmbeddingEngine for DeterministicEmbeddingEngine {
    fn embed(&self, text: &str) -> AppResult<Vec<f32>> {
        let mut vec = vec![0.0f32; self.dim];
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(vec);
        }

        let chars: Vec<char> = trimmed.chars().collect();
        let mut buf = [0u8; 4];

        // 1. Unigram 特征 (权重 1.0)
        for &c in &chars {
            let encoded = c.encode_utf8(&mut buf);
            let h = fnv1a_hash(encoded.as_bytes(), 0x811c9dc5 ^ 0x1111);
            let idx = (h as usize) % self.dim;
            vec[idx] += 1.0;
        }

        // 2. Bigram 特征 (权重 3.0，词组重要性更高)
        for window in chars.windows(2) {
            let mut s = String::with_capacity(8);
            s.push(window[0]);
            s.push(window[1]);
            let h = fnv1a_hash(s.as_bytes(), 0x811c9dc5 ^ 0x2222);
            let idx = (h as usize) % self.dim;
            vec[idx] += 3.0;
        }

        // 3. Trigram 特征 (权重 5.0，短语精确性最高)
        for window in chars.windows(3) {
            let mut s = String::with_capacity(12);
            s.push(window[0]);
            s.push(window[1]);
            s.push(window[2]);
            let h = fnv1a_hash(s.as_bytes(), 0x811c9dc5 ^ 0x3333);
            let idx = (h as usize) % self.dim;
            vec[idx] += 5.0;
        }

        // 4. 空格分词 (若存在独立英文单词或显式分词，权重 4.0)
        for word in trimmed.split_whitespace() {
            if word.chars().count() > 3 {
                let h = fnv1a_hash(word.as_bytes(), 0x811c9dc5 ^ 0x4444);
                let idx = (h as usize) % self.dim;
                vec[idx] += 4.0;
            }
        }

        normalize_in_place(&mut vec);
        Ok(vec)
    }

    fn dimension(&self) -> usize {
        self.dim
    }

    fn name(&self) -> &'static str {
        "deterministic-ngram-fnv1a"
    }
}

pub type SharedEmbeddingEngine = std::sync::Arc<dyn EmbeddingEngine>;
