//! 记忆动力学模块 (FSRS 记忆衰减、强化与自适应遗忘)

pub mod fsrs;
pub mod manager;

pub use fsrs::{
    extraction_strength, retrievability, surprise_bonus, update_difficulty, update_stability,
    RecallRating, DECAY, FACTOR,
};
pub use manager::{DecayReport, MemoryDynamicsManager, DEFAULT_FORGET_THRESHOLD};
