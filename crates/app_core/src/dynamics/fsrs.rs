//! FSRS (Free Spaced Repetition Scheduler) 记忆动力学模型
//!
//! 实现了基于时间衰减的可提取度 R、遗忘临界唤醒强化更新 S、难度动态调整 D
//! 以及间歇效应（Spacing Effect）带来的惊喜唤醒增益（Surprise Bonus）。

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// FSRS 核心算法常数 (借鉴 HIMF 与 Anki FSRS-4.5 标准参数)
// ---------------------------------------------------------------------------
pub const DECAY: f64 = -0.1542;
pub const FACTOR: f64 = 0.980644; // 0.9^(1 / DECAY) - 1.0

pub const W6: f64 = 3.0194; // 难度单步步长
pub const W7: f64 = 0.001; // 均值回归衰减 (回归基准 3.5)
pub const W8: f64 = 1.8722; // 稳定性跃升基础系数
pub const W9: f64 = 0.1666; // 稳定性收益递减指数
pub const W10: f64 = 0.796; // 遗忘边缘强化系数
pub const W15: f64 = 0.6014; // "艰难唤醒 (Hard)" 稳定性增长惩罚系数

/// 记忆唤醒/复习评分等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RecallRating {
    /// 彻底遗忘 (Rating = 1)
    Again,
    /// 艰难记起 (Rating = 2)
    Hard,
    /// 正常唤醒 (Rating = 3)
    #[default]
    Good,
    /// 毫不费力 / 深刻记忆 (Rating = 4)
    Easy,
}

/// 计算特定时间流逝后的记忆可提取度 R ∈ [0.0, 1.0]
///
/// R(t, S) = (1 + FACTOR * (t / S)) ^ DECAY
///
/// * `elapsed_days`: 距上次激活/回忆经过的天数 (浮点数，可精确到小时/分钟)
/// * `stability`: 当前记忆稳定性 S (以天为单位)
pub fn retrievability(elapsed_days: f64, stability: f64) -> f64 {
    let t = elapsed_days.max(0.0);
    let s = stability.max(0.01);

    let base = 1.0 + FACTOR * (t / s);
    if base <= 0.0 {
        return 0.0001;
    }
    base.powf(DECAY).clamp(0.0, 1.0)
}

/// 记忆被成功召回后的稳定性 S' 跃迁更新
///
/// S' = S * (1 + exp(w8) * (11 - D) * S^(-w9) * (exp((1 - R) * w10) - 1) * penalty * bonus)
///
/// **认知动力学特性**：
/// 当 (1 - R) 越大（即在记忆濒临遗忘的边缘被突然记起）时，exp((1-R)*w10) - 1 越大，
/// 稳定性的跃升幅度具有指数级放大效果（间隔重复效应）。
pub fn update_stability(
    old_stability: f64,
    difficulty: f64,
    elapsed_days: f64,
    rating: RecallRating,
) -> f64 {
    let s = old_stability.max(0.01);
    let d = difficulty.clamp(1.0, 10.0);
    let r = retrievability(elapsed_days, s);

    if rating == RecallRating::Again {
        // 若完全遗忘，稳定性断崖式下跌，但仍保留基础残留
        return (s * 0.3).max(0.2);
    }

    let penalty = if rating == RecallRating::Hard {
        W15
    } else {
        1.0
    };
    let bonus = if rating == RecallRating::Easy {
        1.3
    } else {
        1.0
    };

    let growth =
        W8.exp() * (11.0 - d) * s.powf(-W9) * (((1.0 - r) * W10).exp() - 1.0) * penalty * bonus;

    (s * (1.0 + growth)).max(0.1)
}

/// 记忆难度 D' 的自适应更新
///
/// D' = D - w6 * (r - 3) [带阻尼衰减与轻量均值回归]
pub fn update_difficulty(old_difficulty: f64, rating: RecallRating) -> f64 {
    let r = match rating {
        RecallRating::Again => 1.0,
        RecallRating::Hard => 2.0,
        RecallRating::Good => 3.0,
        RecallRating::Easy => 4.0,
    };

    let delta = -W6 * (r - 3.0);
    // 线性阻尼：当难度逼近 10 时，上升变得困难；逼近 1 时下降变缓
    let damped = old_difficulty + delta * (10.0 - old_difficulty) / 9.0;
    // 微弱均值回归至基线 3.5
    let reverted = W7 * 3.5 + (1.0 - W7) * damped;

    reverted.clamp(1.0, 10.0)
}

/// 间隔效应的“惊喜唤醒增益”（Surprise Bonus）
///
/// 在 R ≈ 0.5 处呈现倒 U 型钟形峰值：
/// 太近的记忆（R ≈ 1.0）唤醒平淡无奇；彻底死去的记忆（R ≈ 0.0）极难唤醒；
/// 唯有处于半遗忘半熟悉（R ≈ 0.5）的最佳时机被触碰，带来的认知共振最强。
pub fn surprise_bonus(r: f64) -> f64 {
    0.3 * r * (1.0 - r) * 4.0
}

/// 动态综合提取强度 (Recency + Surprise)
pub fn extraction_strength(elapsed_days: f64, stability: f64) -> f64 {
    let r = retrievability(elapsed_days, stability);
    r + surprise_bonus(r)
}
