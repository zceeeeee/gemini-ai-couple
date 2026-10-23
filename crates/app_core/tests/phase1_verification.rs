use std::sync::Arc;
use app_core::embedding::{DeterministicEmbeddingEngine, EmbeddingEngine};
use app_core::models::{
    L0Conversation, L1EpisodicMemory, L2SemanticFact, L3Narrative, MessageRole,
};
use app_core::retrieval::HybridRetriever;
use app_core::storage::Database;

#[test]
fn test_phase1_storage_and_hybrid_retrieval() {
    // 1. 初始化纯内存 SQLite 存储
    let db = Database::open_in_memory().expect("Failed to initialize database");
    let embedding_engine = Arc::new(DeterministicEmbeddingEngine::default());

    // 2. 验证 L0: 工作记忆
    let conv = L0Conversation {
        id: None,
        session_id: "test_session".to_string(),
        role: MessageRole::User,
        content: "你好，请记住我平时最爱喝乌龙茶。".to_string(),
        created_at: 1000,
        metadata: None,
    };
    let conv_id = db.add_conversation(&conv).expect("Failed to add L0");
    assert!(conv_id > 0);
    let recents = db
        .get_recent_conversations("test_session", 10)
        .expect("Failed to get L0");
    assert_eq!(recents.len(), 1);
    assert_eq!(recents[0].content, conv.content);

    // 3. 验证 L1: 情景记忆与向量化
    let text1 = "用户提到自己平时最爱喝乌龙茶，尤其是少冰无糖。";
    let emb1 = embedding_engine.embed(text1).expect("Embed text1");
    let mut mem1 = L1EpisodicMemory::default();
    mem1.session_id = "test_session".to_string();
    mem1.content = text1.to_string();
    mem1.embedding = Some(emb1.clone());
    mem1.emotion = Some("warm".to_string());
    let mem1_id = db.add_episodic_memory(&mem1).expect("Failed to add L1 text1");
    assert!(mem1_id > 0);

    let text2 = "今天下午三点用户去科技园区参加了技术面试。";
    let emb2 = embedding_engine.embed(text2).expect("Embed text2");
    let mut mem2 = L1EpisodicMemory::default();
    mem2.session_id = "test_session".to_string();
    mem2.content = text2.to_string();
    mem2.embedding = Some(emb2.clone());
    db.add_episodic_memory(&mem2).expect("Failed to add L1 text2");

    // 4. 验证 L2: 语义事实与 FTS5 触发器同步
    let fact = L2SemanticFact {
        id: None,
        entity: "用户".to_string(),
        attribute: "喜好饮品".to_string(),
        value: "乌龙茶少冰".to_string(),
        confidence: 0.95,
        valid_from: None,
        valid_to: None,
        source_event_id: Some(mem1_id),
        created_at: 1000,
        updated_at: 1000,
    };
    db.add_semantic_fact(&fact).expect("Failed to add L2");

    // 5. 验证 L3: 叙事记忆
    let narrative = L3Narrative {
        id: None,
        period_type: "daily".to_string(),
        period_key: "2026-10-23".to_string(),
        summary: "初次相识，了解到用户的饮品喜好。".to_string(),
        reflection: Some("用户偏好清晰，沟通温和。".to_string()),
        created_at: 1000,
        updated_at: 1000,
    };
    db.upsert_narrative(&narrative).expect("Failed to add L3");

    // 6. 验证 L4: 核心画像
    db.set_profile("user_core", "name", "小明").expect("Failed to set L4");
    let profile_val = db
        .get_profile_value("name")
        .expect("Failed to get L4")
        .expect("Profile name should exist");
    assert_eq!(profile_val, "小明");

    // 7. 核心验证：RRF 混合检索（向量 + FTS5 BM25）
    // 注意：查询语句中不包含答案词“乌龙茶”，验证纯意图检索（“用户平时最爱喝什么茶”）
    let retriever = HybridRetriever::new(db, embedding_engine.clone(), None);
    let query = "用户平时最爱喝什么茶";

    let results = retriever
        .retrieve("test_session", query, 5, 5)
        .expect("Hybrid retrieval failed");

    // 验证 L1 情景记忆中与“乌龙茶”相关的条目排在首位
    assert!(!results.episodic_memories.is_empty(), "Episodic memories should not be empty");
    let top_episodic = &results.episodic_memories[0];
    assert!(
        top_episodic.memory.content.contains("乌龙茶"),
        "Top recalled episodic memory should mention 乌龙茶, got: {}",
        top_episodic.memory.content
    );
    assert!(top_episodic.rrf_score > 0.0);

    // 验证 L2 语义事实召回
    assert!(!results.semantic_facts.is_empty(), "Semantic facts should not be empty");
    assert_eq!(results.semantic_facts[0].fact.entity, "用户");
    assert_eq!(results.semantic_facts[0].fact.attribute, "喜好饮品");
    assert_eq!(results.semantic_facts[0].fact.value, "乌龙茶少冰");
}
