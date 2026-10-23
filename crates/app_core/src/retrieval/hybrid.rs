//! 杂化检索器 (Hybrid Retriever)
//!
//! 整合向量空间余弦相似度与 SQLite FTS5 BM25 词法检索，
//! 借助 RRF 互惠排名进行最终排序。

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::embedding::{cosine_similarity, SharedEmbeddingEngine};
use crate::error::AppResult;
use crate::models::{L1EpisodicMemory, L2SemanticFact};
use crate::storage::Database;
use super::rrf::{reciprocal_rank_fusion, RrfConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredEpisodicMemory {
    pub memory: L1EpisodicMemory,
    pub rrf_score: f64,
    pub vector_similarity: Option<f32>,
    pub vector_rank: Option<usize>,
    pub lexical_rank: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredSemanticFact {
    pub fact: L2SemanticFact,
    pub bm25_rank: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridRetrievalResult {
    pub episodic_memories: Vec<ScoredEpisodicMemory>,
    pub semantic_facts: Vec<ScoredSemanticFact>,
}

pub struct HybridRetriever {
    db: Database,
    embedding_engine: SharedEmbeddingEngine,
    config: RrfConfig,
}

impl HybridRetriever {
    pub fn new(
        db: Database,
        embedding_engine: SharedEmbeddingEngine,
        config: Option<RrfConfig>,
    ) -> Self {
        Self {
            db,
            embedding_engine,
            config: config.unwrap_or_default(),
        }
    }

    /// 针对 L1 情景记忆执行向量与 FTS5 双路召回及 RRF 融合排序
    pub fn retrieve_episodic(
        &self,
        session_id: &str,
        query: &str,
        limit: usize,
    ) -> AppResult<Vec<ScoredEpisodicMemory>> {
        // 1. 向量路召回 (Vector Candidate Channel)
        let query_emb = self.embedding_engine.embed(query)?;
        let all_active = self.db.get_active_episodic_memories(session_id)?;

        let mut vector_candidates: Vec<(i64, f32)> = Vec::new();
        let mut memory_map: HashMap<i64, L1EpisodicMemory> = HashMap::new();

        for m in all_active {
            if let Some(id) = m.id {
                if let Some(emb) = &m.embedding {
                    let sim = cosine_similarity(&query_emb, emb);
                    vector_candidates.push((id, sim));
                }
                memory_map.insert(id, m);
            }
        }

        // 按余弦相似度降序排序
        vector_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let vector_ranked_ids: Vec<i64> = vector_candidates.iter().map(|(id, _)| *id).collect();
        let vector_sim_map: HashMap<i64, f32> = vector_candidates.into_iter().collect();

        // 2. 词法路召回 (FTS5 BM25 Channel)
        let fts_candidates = self.db.search_episodic_fts(query, limit * 2)?;
        let mut lexical_ranked_ids: Vec<i64> = Vec::new();
        for (m, _rank) in fts_candidates {
            if let Some(id) = m.id {
                lexical_ranked_ids.push(id);
                memory_map.entry(id).or_insert(m);
            }
        }

        // 3. 执行 RRF 互惠排名融合
        let fusion_results =
            reciprocal_rank_fusion(&vector_ranked_ids, &lexical_ranked_ids, &self.config);

        // 4. 组装结果并按 Top-K 截断
        let mut scored_list = Vec::new();
        for item in fusion_results.into_iter().take(limit) {
            if let Some(memory) = memory_map.remove(&item.id) {
                let sim = vector_sim_map.get(&item.id).copied();
                scored_list.push(ScoredEpisodicMemory {
                    memory,
                    rrf_score: item.rrf_score,
                    vector_similarity: sim,
                    vector_rank: item.vector_rank,
                    lexical_rank: item.lexical_rank,
                });
            }
        }

        Ok(scored_list)
    }

    /// 针对 L2 语义事实执行 FTS5 召回
    pub fn retrieve_semantic(&self, query: &str, limit: usize) -> AppResult<Vec<ScoredSemanticFact>> {
        let list = self.db.search_semantic_fts(query, limit)?;
        let scored = list
            .into_iter()
            .map(|(fact, rank)| ScoredSemanticFact {
                fact,
                bm25_rank: rank,
            })
            .collect();
        Ok(scored)
    }

    /// 综合检索入口：同步召回 L1 情景记忆与 L2 语义事实
    pub fn retrieve(
        &self,
        session_id: &str,
        query: &str,
        episodic_limit: usize,
        semantic_limit: usize,
    ) -> AppResult<HybridRetrievalResult> {
        let episodic = self.retrieve_episodic(session_id, query, episodic_limit)?;
        let semantic = self.retrieve_semantic(query, semantic_limit)?;

        Ok(HybridRetrievalResult {
            episodic_memories: episodic,
            semantic_facts: semantic,
        })
    }
}
