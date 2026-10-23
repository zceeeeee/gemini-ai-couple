use chrono::Utc;
use serde::{Deserialize, Serialize};

/// 消息角色定义
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Assistant => write!(f, "assistant"),
            Self::System => write!(f, "system"),
        }
    }
}

impl std::str::FromStr for MessageRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" => Ok(Self::User),
            "assistant" => Ok(Self::Assistant),
            "system" => Ok(Self::System),
            other => Err(format!("Unknown message role: {}", other)),
        }
    }
}

/// L0 工作记忆条目（原始滑动窗口对话）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L0Conversation {
    pub id: Option<i64>,
    pub session_id: String,
    pub role: MessageRole,
    pub content: String,
    pub created_at: i64,
    pub metadata: Option<String>,
}

/// L1 情景记忆条目（具体事件、情绪与 FSRS 参数）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L1EpisodicMemory {
    pub id: Option<i64>,
    pub session_id: String,
    pub content: String,
    pub emotion: Option<String>,
    pub embedding: Option<Vec<f32>>,
    // FSRS 记忆动力学参数
    pub stability: f64,
    pub difficulty: f64,
    pub retrievability: f64,
    pub last_recalled_at: i64,
    pub recall_count: u32,
    pub status: String, // "active", "archived"
    pub created_at: i64,
    pub updated_at: i64,
    pub metadata: Option<String>,
}

impl Default for L1EpisodicMemory {
    fn default() -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            id: None,
            session_id: "default".to_string(),
            content: String::new(),
            emotion: None,
            embedding: None,
            stability: 1.0,
            difficulty: 5.0,
            retrievability: 1.0,
            last_recalled_at: now,
            recall_count: 0,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
            metadata: None,
        }
    }
}

/// L2 语义事实条目（实体-属性-值三元组）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2SemanticFact {
    pub id: Option<i64>,
    pub entity: String,
    pub attribute: String,
    pub value: String,
    pub confidence: f64,
    pub valid_from: Option<i64>,
    pub valid_to: Option<i64>,
    pub source_event_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Default for L2SemanticFact {
    fn default() -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            id: None,
            entity: String::new(),
            attribute: String::new(),
            value: String::new(),
            confidence: 1.0,
            valid_from: None,
            valid_to: None,
            source_event_id: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// L3 叙事记忆条目（阶段回顾与时间线日记）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L3Narrative {
    pub id: Option<i64>,
    pub period_type: String, // "daily", "weekly", "milestone"
    pub period_key: String,  // e.g. "2026-10-23"
    pub summary: String,
    pub reflection: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// L4 核心画像条目（不可磨灭底色）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L4Profile {
    pub id: Option<i64>,
    pub category: String, // "user_core", "companion_persona", "relationship"
    pub key: String,
    pub value: String,
    pub created_at: i64,
    pub updated_at: i64,
}
