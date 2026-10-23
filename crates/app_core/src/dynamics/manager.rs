//! 记忆动力学管理器 (MemoryDynamicsManager)
//!
//! 负责执行批量时间衰减、临界记忆遗忘归档、召回强化以及动态检索加权。

use serde::{Deserialize, Serialize};
use tracing::info;

use super::fsrs::{
    extraction_strength, retrievability, update_difficulty, update_stability, RecallRating,
};
use crate::error::{AppError, AppResult};
use crate::models::L1EpisodicMemory;
use crate::storage::Database;

/// 遗忘归档的默认临界阈值 (R < 0.35 即被认定为潜意识沉睡/沉淀归档)
pub const DEFAULT_FORGET_THRESHOLD: f64 = 0.35;

/// 衰减报告统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayReport {
    pub total_evaluated: usize,
    pub archived_count: usize,
    pub active_remaining: usize,
}

#[derive(Clone)]
pub struct MemoryDynamicsManager {
    db: Database,
}

impl MemoryDynamicsManager {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// 执行全局或会话级的时间流逝记忆衰减
    ///
    /// * `session_id`: 目标会话 ID
    /// * `now_millis`: 当前时间戳 (毫秒)，支持测试时进行时间快进 (Time Travel)
    /// * `forget_threshold`: 遗忘归档阈值，若为 None 则使用默认值 0.15
    pub fn apply_time_decay(
        &self,
        session_id: &str,
        now_millis: i64,
        forget_threshold: Option<f64>,
    ) -> AppResult<DecayReport> {
        let threshold = forget_threshold.unwrap_or(DEFAULT_FORGET_THRESHOLD);
        let active_memories = self.db.get_active_episodic_memories(session_id)?;

        let mut archived_count = 0;
        let mut active_remaining = 0;
        let total_evaluated = active_memories.len();

        for mem in &active_memories {
            let Some(id) = mem.id else { continue };

            let elapsed_millis = (now_millis - mem.last_recalled_at).max(0);
            let elapsed_days = elapsed_millis as f64 / (1000.0 * 86400.0);

            let new_r = retrievability(elapsed_days, mem.stability);
            let is_archived = new_r < threshold;
            let status = if is_archived {
                archived_count += 1;
                "archived"
            } else {
                active_remaining += 1;
                "active"
            };

            self.db.update_episodic_fsrs(
                id,
                mem.stability,
                mem.difficulty,
                new_r,
                mem.last_recalled_at,
                mem.recall_count,
                status,
                now_millis,
            )?;
        }

        info!(
            "Memory time decay applied: evaluated={}, archived={}, active_remaining={}",
            total_evaluated, archived_count, active_remaining
        );

        Ok(DecayReport {
            total_evaluated,
            archived_count,
            active_remaining,
        })
    }

    /// 当用户或伴侣唤醒/提及该情景记忆时，触发强化更新
    ///
    /// * `memory_id`: 情景记忆 ID
    /// * `rating`: 唤醒评价 (Again, Hard, Good, Easy)
    /// * `now_millis`: 唤醒发生时的时间戳
    pub fn reinforce_memory(
        &self,
        memory_id: i64,
        rating: RecallRating,
        now_millis: i64,
    ) -> AppResult<L1EpisodicMemory> {
        let mem = self
            .db
            .get_episodic_memory_by_id(memory_id)?
            .ok_or_else(|| AppError::NotFound(format!("Memory id {} not found", memory_id)))?;

        let elapsed_millis = (now_millis - mem.last_recalled_at).max(0);
        let elapsed_days = elapsed_millis as f64 / (1000.0 * 86400.0);

        let new_s = update_stability(mem.stability, mem.difficulty, elapsed_days, rating);
        let new_d = update_difficulty(mem.difficulty, rating);
        let new_r = 1.0; // 刚被唤醒，可提取度重置为 1.0
        let new_count = mem.recall_count + 1;
        let new_status = "active";

        self.db.update_episodic_fsrs(
            memory_id, new_s, new_d, new_r, now_millis, new_count, new_status, now_millis,
        )?;

        let mut updated = mem;
        updated.stability = new_s;
        updated.difficulty = new_d;
        updated.retrievability = new_r;
        updated.recall_count = new_count;
        updated.last_recalled_at = now_millis;
        updated.updated_at = now_millis;
        updated.status = new_status.to_string();

        Ok(updated)
    }

    /// 计算带记忆动力学生命力加权的分数
    pub fn calculate_dynamic_score(
        &self,
        base_rrf_score: f64,
        memory: &L1EpisodicMemory,
        now_millis: i64,
    ) -> f64 {
        let elapsed_millis = (now_millis - memory.last_recalled_at).max(0);
        let elapsed_days = elapsed_millis as f64 / (1000.0 * 86400.0);

        let strength = extraction_strength(elapsed_days, memory.stability);
        // 动态加权公式：在 RRF 基础上，乘以 (0.75 + 0.5 * extraction_strength)
        base_rrf_score * (0.75 + 0.5 * strength)
    }
}
