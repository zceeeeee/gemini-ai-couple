use app_core::consolidation::{ConsolidationWorker, RuleBasedConsolidationExtractor};
use app_core::dynamics::{retrievability, surprise_bonus, MemoryDynamicsManager, RecallRating};
use app_core::embedding::DeterministicEmbeddingEngine;
use app_core::models::{L0Conversation, L1EpisodicMemory, MessageRole};
use app_core::retrieval::HybridRetriever;
use app_core::storage::Database;
use std::sync::Arc;

#[test]
fn test_phase2_fsrs_and_sleep_consolidation() {
    let now: i64 = 1700000000000; // 模拟基准时间戳
    let day_ms = 86400 * 1000;

    // =========================================================================
    // 1. 验证 FSRS 动力学数学模型 (衰减曲线、惊喜增益、临界强化)
    // =========================================================================
    let s0 = 1.0;
    let r_day0 = retrievability(0.0, s0);
    let r_day1 = retrievability(1.0, s0);
    let r_day7 = retrievability(7.0, s0);
    let r_day30 = retrievability(30.0, s0);

    assert!(
        (r_day0 - 1.0).abs() < 1e-4,
        "Day 0 retrievability should be 1.0"
    );
    assert!(
        r_day1 < r_day0 && r_day1 > 0.85,
        "Day 1 retrievability should decay slightly (~0.89), got: {}",
        r_day1
    );
    assert!(
        r_day7 < r_day1 && r_day7 > 0.70,
        "Day 7 retrievability should be ~0.72, got: {}",
        r_day7
    );
    assert!(
        r_day30 < r_day7 && r_day30 > 0.50,
        "Day 30 retrievability should be ~0.58, got: {}",
        r_day30
    );

    // 验证惊喜唤醒增益 (Surprise Bonus 在 R ≈ 0.5 时呈现倒 U 型峰值)
    let surprise_peak = surprise_bonus(0.5);
    let surprise_fresh = surprise_bonus(1.0);
    let surprise_dead = surprise_bonus(0.0);
    assert!(
        (surprise_peak - 0.3).abs() < 1e-4,
        "Peak surprise should be 0.3"
    );
    assert_eq!(surprise_fresh, 0.0, "Fresh memory has 0 surprise bonus");
    assert_eq!(
        surprise_dead, 0.0,
        "Completely forgotten memory has 0 surprise bonus"
    );

    // =========================================================================
    // 2. 验证 Database 集成、时间衰减与自适应遗忘归档 (Threshold Archiving)
    // =========================================================================
    let db = Database::open_in_memory().expect("Initialize DB");
    let dynamics_mgr = MemoryDynamicsManager::new(db.clone());

    // 插入一条高稳定性的深刻记忆
    let strong_mem = L1EpisodicMemory {
        session_id: "test_user".to_string(),
        content: "用户与伴侣共同在海边看了浪漫日落。".to_string(),
        stability: 10.0,
        last_recalled_at: now,
        ..Default::default()
    };
    let strong_id = db.add_episodic_memory(&strong_mem).expect("Add strong mem");

    // 插入一条极低稳定性的碎片记忆
    let weak_mem = L1EpisodicMemory {
        session_id: "test_user".to_string(),
        content: "用户随口提到刚才路过了一家便利店。".to_string(),
        stability: 0.02, // 极易遗忘
        last_recalled_at: now,
        ..Default::default()
    };
    let weak_id = db.add_episodic_memory(&weak_mem).expect("Add weak mem");

    // 时间跃迁快进 60 天
    let time_after_60_days = now + 60 * day_ms;
    let decay_report = dynamics_mgr
        .apply_time_decay("test_user", time_after_60_days, None)
        .expect("Apply decay");

    assert_eq!(decay_report.total_evaluated, 2);
    assert_eq!(
        decay_report.archived_count, 1,
        "Weak memory should be archived"
    );
    assert_eq!(
        decay_report.active_remaining, 1,
        "Strong memory should remain active"
    );

    let active_after_decay = db
        .get_active_episodic_memories("test_user")
        .expect("Get active");
    assert_eq!(active_after_decay.len(), 1);
    assert_eq!(active_after_decay[0].id, Some(strong_id));

    let archived_weak = db
        .get_episodic_memory_by_id(weak_id)
        .expect("Get weak")
        .expect("Should exist");
    assert_eq!(archived_weak.status, "archived");

    // =========================================================================
    // 3. 验证遗忘边缘的成功唤醒强化 (Recall Reinforcement)
    // =========================================================================
    let reinforced = dynamics_mgr
        .reinforce_memory(strong_id, RecallRating::Good, time_after_60_days)
        .expect("Reinforce memory");

    assert_eq!(reinforced.recall_count, 1);
    assert_eq!(reinforced.retrievability, 1.0);
    assert!(
        reinforced.stability > 10.0,
        "Stability should grow significantly after timely recall, got: {}",
        reinforced.stability
    );

    // =========================================================================
    // 4. 验证睡眠巩固流水线 (L0 -> L1/L2/L3) 与 矛盾消歧 (Contradiction Resolution)
    // =========================================================================
    let embedding_engine = Arc::new(DeterministicEmbeddingEngine::default());
    let extractor = Arc::new(RuleBasedConsolidationExtractor);
    let worker = ConsolidationWorker::new(db.clone(), embedding_engine.clone(), extractor);

    // 4.1 写入首批 L0 工作对话 (提及爱喝乌龙茶与技术面试)
    let conv1 = L0Conversation {
        id: None,
        session_id: "test_user".to_string(),
        role: MessageRole::User,
        content: "你好呀，我平时最爱喝乌龙茶。".to_string(),
        created_at: now,
        metadata: None,
    };
    let conv2 = L0Conversation {
        id: None,
        session_id: "test_user".to_string(),
        role: MessageRole::User,
        content: "今天下午还要去科技园区参加技术面试，有点小紧张。".to_string(),
        created_at: now + 1000,
        metadata: None,
    };
    db.add_conversation(&conv1).expect("Add conv1");
    db.add_conversation(&conv2).expect("Add conv2");

    // 执行首次睡眠巩固
    let report1 = worker
        .consolidate_session("test_user", 10, now + 2000)
        .expect("Consolidate session 1");

    assert_eq!(report1.processed_conversations, 2);
    assert!(report1.created_episodic_count >= 1);
    assert_eq!(report1.created_facts_count, 1);
    assert_eq!(report1.invalidated_facts_count, 0);
    assert!(report1.created_narrative);

    // 验证 L0 对话被标记为已沉淀
    let unconsolidated_left = db
        .get_unconsolidated_conversations("test_user", 10)
        .expect("Check left");
    assert!(
        unconsolidated_left.is_empty(),
        "All conversations should be consolidated"
    );

    // 验证 L2 事实当前为乌龙茶
    let active_facts1 = db.get_active_semantic_facts().expect("Get active facts");
    assert_eq!(active_facts1.len(), 1);
    assert_eq!(active_facts1[0].entity, "用户");
    assert_eq!(active_facts1[0].attribute, "喜好饮品");
    assert_eq!(active_facts1[0].value, "乌龙茶");

    // 4.2 验证矛盾消歧：用户偏好更新 (Reconsolidation / Contradiction Resolution)
    let conv3 = L0Conversation {
        id: None,
        session_id: "test_user".to_string(),
        role: MessageRole::User,
        content: "其实我现在改喝茉莉花茶了，不爱喝乌龙茶了。".to_string(),
        created_at: now + 5000,
        metadata: None,
    };
    db.add_conversation(&conv3).expect("Add conv3");

    let report2 = worker
        .consolidate_session("test_user", 10, now + 6000)
        .expect("Consolidate session 2");

    assert_eq!(report2.processed_conversations, 1);
    assert_eq!(report2.created_facts_count, 1);
    assert_eq!(
        report2.invalidated_facts_count, 1,
        "Old tea preference must be invalidated"
    );

    // 验证活跃事实已自动切换为茉莉花茶
    let active_facts2 = db.get_active_semantic_facts().expect("Get active facts 2");
    assert_eq!(active_facts2.len(), 1);
    assert_eq!(active_facts2[0].value, "茉莉花茶");
    assert!(active_facts2[0].valid_to.is_none());

    // 验证所有事实中包含旧事实历史，且其 valid_to 已被标记
    let all_facts = db.get_all_semantic_facts().expect("Get all facts");
    assert_eq!(all_facts.len(), 2);
    let old_fact = all_facts
        .iter()
        .find(|f| f.value == "乌龙茶")
        .expect("Old fact should exist in history");
    assert!(
        old_fact.valid_to.is_some(),
        "Old fact valid_to should be set for non-destructive history"
    );

    // =========================================================================
    // 5. 验证带动力学加权的杂化检索 (HybridRetriever + FSRS Dynamics)
    // =========================================================================
    let retriever = HybridRetriever::new(db.clone(), embedding_engine.clone(), None);
    let retrieved_with_dynamics = retriever
        .retrieve_episodic_with_dynamics("test_user", "重要技术面试", 5, now + 6000, &dynamics_mgr)
        .expect("Retrieve with dynamics");

    assert!(!retrieved_with_dynamics.is_empty());
    assert!(retrieved_with_dynamics[0].memory.content.contains("面试"));
    assert!(retrieved_with_dynamics[0].rrf_score > 0.0);
}
