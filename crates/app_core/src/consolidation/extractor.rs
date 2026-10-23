//! 记忆沉淀提取器抽象 (Consolidation Extractor)
//!
//! 将零散的 L0 工作记忆对话重放、提炼并升华为 L1 情景记忆、L2 语义事实与 L3 叙事反思。

use crate::error::AppResult;
use crate::models::{L0Conversation, MessageRole};
use serde::{Deserialize, Serialize};

/// 沉淀生成的情景记忆草稿
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftEpisodicMemory {
    pub content: String,
    pub emotion: Option<String>,
}

/// 沉淀生成的语义事实草稿 (实体-属性-值三元组)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftFact {
    pub entity: String,
    pub attribute: String,
    pub value: String,
    pub confidence: f64,
}

/// 沉淀生成的叙事记忆草稿
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftNarrative {
    pub period_type: String,
    pub period_key: String,
    pub summary: String,
    pub reflection: Option<String>,
}

/// 睡眠巩固输出草案包
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConsolidationDraft {
    pub episodic_memories: Vec<DraftEpisodicMemory>,
    pub facts: Vec<DraftFact>,
    pub narrative: Option<DraftNarrative>,
}

/// 记忆巩固提取器接口
///
/// 生产环境中可接入 LLM (如 Gemini/DeepSeek/OpenAI/本地小模型)，
/// 在测试与离线环境下使用启发式规则引擎。
pub trait ConsolidationExtractor: Send + Sync {
    fn extract(&self, conversations: &[L0Conversation]) -> AppResult<ConsolidationDraft>;
}

/// 基于确定性启发式规则的巩固提取器 (支持零依赖测试验证)
#[derive(Default)]
pub struct RuleBasedConsolidationExtractor;

impl ConsolidationExtractor for RuleBasedConsolidationExtractor {
    fn extract(&self, conversations: &[L0Conversation]) -> AppResult<ConsolidationDraft> {
        let mut draft = ConsolidationDraft::default();
        if conversations.is_empty() {
            return Ok(draft);
        }

        let mut user_dialogues = Vec::new();
        let mut has_interview = false;
        let mut drink_pref = None;
        let mut revision_pref = None;

        for conv in conversations {
            if conv.role == MessageRole::User {
                user_dialogues.push(conv.content.as_str());

                // 启发式检测偏好与喜好
                if conv.content.contains("最爱喝") || conv.content.contains("喜欢喝") {
                    if let Some(pos) = conv.content.find("喝") {
                        let after = &conv.content[pos + 3..];
                        if let Some(token) = extract_target_phrase(after) {
                            drink_pref = Some(token);
                        }
                    }
                }

                // 启发式检测偏好修正 / 矛盾更新 (HIMF Reconsolidation 模式)
                if conv.content.contains("改喝")
                    || conv.content.contains("现在喝")
                    || conv.content.contains("其实现在喜欢")
                {
                    if let Some(pos) = conv.content.find("喝") {
                        let after = &conv.content[pos + 3..];
                        if let Some(token) = extract_target_phrase(after) {
                            revision_pref = Some(token);
                        }
                    }
                }

                if conv.content.contains("面试") {
                    has_interview = true;
                }
            }
        }

        // 1. 生成 L2 语义事实
        if let Some(rev) = revision_pref {
            draft.facts.push(DraftFact {
                entity: "用户".to_string(),
                attribute: "喜好饮品".to_string(),
                value: rev,
                confidence: 0.98,
            });
        } else if let Some(drink) = drink_pref {
            draft.facts.push(DraftFact {
                entity: "用户".to_string(),
                attribute: "喜好饮品".to_string(),
                value: drink,
                confidence: 0.95,
            });
        }

        // 2. 生成 L1 情景记忆
        if has_interview {
            draft.episodic_memories.push(DraftEpisodicMemory {
                content: "用户下午去科技园区参加了重要技术面试。".to_string(),
                emotion: Some("nervous_hopeful".to_string()),
            });
        }

        if let Some(fact) = draft.facts.first() {
            draft.episodic_memories.push(DraftEpisodicMemory {
                content: format!("用户分享了关于{}的偏好：{}", fact.attribute, fact.value),
                emotion: Some("peaceful".to_string()),
            });
        }

        // 3. 生成 L3 叙事总结
        let summary_text = format!(
            "本轮交流共计 {} 轮对话，提炼出关键动态与偏好更新。",
            conversations.len()
        );
        draft.narrative = Some(DraftNarrative {
            period_type: "daily".to_string(),
            period_key: "consolidation_session".to_string(),
            summary: summary_text,
            reflection: Some("用户分享了生活细节，关系在自然交互中加深。".to_string()),
        });

        Ok(draft)
    }
}

/// 辅助提取短语目标实体/属性值，在分界词（标点、语气词等）处截断
fn extract_target_phrase(input: &str) -> Option<String> {
    let delimiters = ['，', '。', '！', '!', ',', '；', ';', '了', '吧', '呢'];
    let clause = input.split(&delimiters[..]).next()?.trim();
    if clause.is_empty() {
        None
    } else {
        Some(clause.to_string())
    }
}
