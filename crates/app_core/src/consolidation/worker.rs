//! 睡眠巩固后台工作流 (Sleep Consolidation Worker)
//!
//! 定期或在闲置时启动，重放并提炼 L0 碎片化工作记忆，
//! 升华入库 L1/L2/L3，并执行事实冲突消歧 (Contradiction Resolution)。

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use super::extractor::ConsolidationExtractor;
use crate::embedding::EmbeddingEngine;
use crate::error::AppResult;
use crate::models::{L1EpisodicMemory, L2SemanticFact, L3Narrative};
use crate::storage::Database;

/// 睡眠巩固执行结果报告
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConsolidationReport {
    pub processed_conversations: usize,
    pub created_episodic_count: usize,
    pub created_facts_count: usize,
    pub invalidated_facts_count: usize,
    pub created_narrative: bool,
}

pub struct ConsolidationWorker {
    db: Database,
    embedding_engine: Arc<dyn EmbeddingEngine>,
    extractor: Arc<dyn ConsolidationExtractor>,
}

impl ConsolidationWorker {
    pub fn new(
        db: Database,
        embedding_engine: Arc<dyn EmbeddingEngine>,
        extractor: Arc<dyn ConsolidationExtractor>,
    ) -> Self {
        Self {
            db,
            embedding_engine,
            extractor,
        }
    }

    /// 触发会话级睡眠巩固
    ///
    /// * `session_id`: 目标会话 ID
    /// * `batch_limit`: 每次巩固提取的最大对话轮数
    /// * `now_millis`: 模拟或当前的巩固时间戳
    pub fn consolidate_session(
        &self,
        session_id: &str,
        batch_limit: usize,
        now_millis: i64,
    ) -> AppResult<ConsolidationReport> {
        let unconsolidated = self
            .db
            .get_unconsolidated_conversations(session_id, batch_limit)?;
        if unconsolidated.is_empty() {
            return Ok(ConsolidationReport::default());
        }

        info!(
            "Starting consolidation for session '{}' with {} conversations",
            session_id,
            unconsolidated.len()
        );

        // 1. 调用提取器抽取草案
        let draft = self.extractor.extract(&unconsolidated)?;

        // 2. 升华写入 L1 情景记忆 (含向量嵌入与 FSRS 初始参数)
        let mut created_episodic_count = 0;
        let mut last_episodic_id = None;

        for ep in &draft.episodic_memories {
            let embedding = self.embedding_engine.embed(&ep.content).ok();
            let mem = L1EpisodicMemory {
                session_id: session_id.to_string(),
                content: ep.content.clone(),
                emotion: ep.emotion.clone(),
                embedding,
                stability: 1.0,
                difficulty: 5.0,
                retrievability: 1.0,
                last_recalled_at: now_millis,
                created_at: now_millis,
                updated_at: now_millis,
                ..Default::default()
            };

            let inserted_id = self.db.add_episodic_memory(&mem)?;
            last_episodic_id = Some(inserted_id);
            created_episodic_count += 1;
        }

        // 3. 升华写入 L2 语义事实并进行矛盾消歧 (Contradiction Resolution)
        let mut created_facts_count = 0;
        let mut invalidated_facts_count = 0;

        for fact in &draft.facts {
            // 检查并软注销发生冲突的旧事实 (将有效终止时间 valid_to 设为当前时间)
            let invalidated = self.db.invalidate_conflicting_facts(
                &fact.entity,
                &fact.attribute,
                &fact.value,
                now_millis,
            )?;
            invalidated_facts_count += invalidated;

            // 写入新的有效事实
            let semantic_fact = L2SemanticFact {
                id: None,
                entity: fact.entity.clone(),
                attribute: fact.attribute.clone(),
                value: fact.value.clone(),
                confidence: fact.confidence,
                valid_from: Some(now_millis),
                valid_to: None,
                source_event_id: last_episodic_id,
                created_at: now_millis,
                updated_at: now_millis,
            };
            self.db.add_semantic_fact(&semantic_fact)?;
            created_facts_count += 1;
        }

        // 4. 写入 L3 叙事总结
        let mut created_narrative = false;
        if let Some(narrative_draft) = &draft.narrative {
            let narrative = L3Narrative {
                id: None,
                period_type: narrative_draft.period_type.clone(),
                period_key: narrative_draft.period_key.clone(),
                summary: narrative_draft.summary.clone(),
                reflection: narrative_draft.reflection.clone(),
                created_at: now_millis,
                updated_at: now_millis,
            };
            self.db.upsert_narrative(&narrative)?;
            created_narrative = true;
        }

        // 5. 标记 L0 对话为已沉淀
        let ids: Vec<i64> = unconsolidated.iter().filter_map(|c| c.id).collect();
        self.db.mark_conversations_consolidated(&ids)?;

        info!(
            "Consolidation complete: processed={}, L1={}, L2={}, invalidated_old={}, L3={}",
            unconsolidated.len(),
            created_episodic_count,
            created_facts_count,
            invalidated_facts_count,
            created_narrative
        );

        Ok(ConsolidationReport {
            processed_conversations: unconsolidated.len(),
            created_episodic_count,
            created_facts_count,
            invalidated_facts_count,
            created_narrative,
        })
    }
}
