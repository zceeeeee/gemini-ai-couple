//! 睡眠巩固模块 (Sleep Consolidation)
//!
//! 负责从碎片化 L0 工作记忆中重放并提炼情景、事实与长程叙事反思，
//! 并在记忆演变时执行知识图谱矛盾消歧与修正。

pub mod extractor;
pub mod worker;

pub use extractor::{
    ConsolidationDraft, ConsolidationExtractor, DraftEpisodicMemory, DraftFact, DraftNarrative,
    RuleBasedConsolidationExtractor,
};
pub use worker::{ConsolidationReport, ConsolidationWorker};
