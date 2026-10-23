//! RRF (Reciprocal Rank Fusion) 互惠排名算法
//!
//! 将不同检索渠道（向量近似检索、FTS5 BM25 词法检索）的排名列表
//! 基于倒数加权融合成统一的综合评分。

use std::collections::HashMap;
use std::hash::Hash;

/// RRF 融合参数配置
#[derive(Debug, Clone)]
pub struct RrfConfig {
    /// 平滑系数 k (常取 60)
    pub k: f64,
    /// 向量检索权重
    pub vector_weight: f64,
    /// 词法/BM25 检索权重
    pub lexical_weight: f64,
}

impl Default for RrfConfig {
    fn default() -> Self {
        Self {
            k: 60.0,
            vector_weight: 0.6,
            lexical_weight: 0.4,
        }
    }
}

/// 单个检索通道的结果项 (包含标识与该通道内的原始得分)
#[derive(Debug, Clone)]
pub struct RankedItem<ID> {
    pub id: ID,
    pub raw_score: f64,
}

/// 融合后的结果项
#[derive(Debug, Clone)]
pub struct FusionResult<ID> {
    pub id: ID,
    pub rrf_score: f64,
    pub vector_rank: Option<usize>,
    pub lexical_rank: Option<usize>,
}

/// 执行 RRF 融合计算
///
/// * `vector_ranked`: 按向量相似度降序排列的 ID 列表 (Rank 1, 2, ...)
/// * `lexical_ranked`: 按词法 BM25 匹配度优选排列的 ID 列表 (Rank 1, 2, ...)
/// * `config`: RRF 参数
pub fn reciprocal_rank_fusion<ID: Eq + Hash + Clone>(
    vector_ranked: &[ID],
    lexical_ranked: &[ID],
    config: &RrfConfig,
) -> Vec<FusionResult<ID>> {
    let mut scores: HashMap<ID, f64> = HashMap::new();
    let mut vec_ranks: HashMap<ID, usize> = HashMap::new();
    let mut lex_ranks: HashMap<ID, usize> = HashMap::new();

    // 1. 处理向量检索排名
    for (i, id) in vector_ranked.iter().enumerate() {
        let rank = i + 1; // 1-based rank
        vec_ranks.insert(id.clone(), rank);
        let score_contribution = config.vector_weight / (config.k + rank as f64);
        *scores.entry(id.clone()).or_insert(0.0) += score_contribution;
    }

    // 2. 处理词法 FTS5 排名
    for (i, id) in lexical_ranked.iter().enumerate() {
        let rank = i + 1; // 1-based rank
        lex_ranks.insert(id.clone(), rank);
        let score_contribution = config.lexical_weight / (config.k + rank as f64);
        *scores.entry(id.clone()).or_insert(0.0) += score_contribution;
    }

    // 3. 收集并按 RRF 得分降序排序
    let mut results: Vec<FusionResult<ID>> = scores
        .into_iter()
        .map(|(id, score)| FusionResult {
            vector_rank: vec_ranks.get(&id).copied(),
            lexical_rank: lex_ranks.get(&id).copied(),
            id,
            rrf_score: score,
        })
        .collect();

    results.sort_by(|a, b| {
        b.rrf_score
            .partial_cmp(&a.rrf_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    results
}
