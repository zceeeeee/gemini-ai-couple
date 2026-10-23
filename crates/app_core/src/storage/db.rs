use std::path::Path;
use std::sync::{Arc, Mutex};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use tracing::{info, warn};

use crate::error::{AppError, AppResult};
use crate::models::{
    L0Conversation, L1EpisodicMemory, L2SemanticFact, L3Narrative, L4Profile, MessageRole,
};
use super::schema::{INIT_SQL_FALLBACK, INIT_SQL_TRIGRAM};

pub fn embedding_to_blob(vec: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vec.len() * 4);
    for val in vec {
        bytes.extend_from_slice(&val.to_le_bytes());
    }
    bytes
}

pub fn blob_to_embedding(bytes: &[u8]) -> Vec<f32> {
    let (chunks, _) = bytes.as_chunks::<4>();
    let mut vec = Vec::with_capacity(chunks.len());
    for &chunk in chunks {
        vec.push(f32::from_le_bytes(chunk));
    }
    vec
}

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// 创建内存数据库连接 (适合测试或纯内存环境)
    pub fn open_in_memory() -> AppResult<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
        Ok(db)
    }

    /// 打开指定文件路径的 SQLite 数据库
    pub fn open<P: AsRef<Path>>(path: P) -> AppResult<Self> {
        if let Some(parent) = path.as_ref().parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| AppError::Internal(format!("Failed to create db dir: {}", e)))?;
            }
        }
        let conn = Connection::open(path)?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
        Ok(db)
    }

    /// 初始化五层存储架构表与 FTS5 虚拟索引
    pub fn init_schema(&self) -> AppResult<()> {
        let conn = self.conn.lock().unwrap();

        // 优先尝试使用 trigram 分词器构建 FTS5
        match conn.execute_batch(INIT_SQL_TRIGRAM) {
            Ok(_) => {
                info!("SQLite schema initialized successfully with trigram FTS5");
            }
            Err(e) => {
                warn!(
                    "Failed to initialize trigram FTS5 (possibly unsupported): {}. Falling back to standard tokenizer.",
                    e
                );
                conn.execute_batch(INIT_SQL_FALLBACK)?;
                info!("SQLite schema initialized successfully with fallback tokenizer");
            }
        }

        // 记录初始化时间戳与版本
        let now = Utc::now().timestamp_millis();
        conn.execute(
            r#"INSERT OR REPLACE INTO schema_meta (key, value, updated_at)
               VALUES ('version', '1.0.0', ?1), ('phase', '1', ?1)"#,
            params![now],
        )?;

        Ok(())
    }

    // ==========================================
    // L0: 工作记忆 (Conversations)
    // ==========================================

    pub fn add_conversation(&self, item: &L0Conversation) -> AppResult<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"INSERT INTO conversations (session_id, role, content, created_at, metadata)
               VALUES (?1, ?2, ?3, ?4, ?5)"#,
            params![
                item.session_id,
                item.role.to_string(),
                item.content,
                item.created_at,
                item.metadata,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_recent_conversations(
        &self,
        session_id: &str,
        limit: usize,
    ) -> AppResult<Vec<L0Conversation>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"SELECT id, session_id, role, content, created_at, metadata
               FROM conversations
               WHERE session_id = ?1
               ORDER BY created_at DESC, id DESC
               LIMIT ?2"#,
        )?;

        let rows = stmt.query_map(params![session_id, limit as i64], |row| {
            let role_str: String = row.get(2)?;
            let role = role_str.parse().unwrap_or(MessageRole::User);
            Ok(L0Conversation {
                id: Some(row.get(0)?),
                session_id: row.get(1)?,
                role,
                content: row.get(3)?,
                created_at: row.get(4)?,
                metadata: row.get(5)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        // 按时间升序返回（适合作为 Prompt 上下文滑动窗口）
        list.reverse();
        Ok(list)
    }

    // ==========================================
    // L1: 情景记忆 (Episodic Memories)
    // ==========================================

    pub fn add_episodic_memory(&self, item: &L1EpisodicMemory) -> AppResult<i64> {
        let conn = self.conn.lock().unwrap();
        let blob = item.embedding.as_ref().map(|v| embedding_to_blob(v));

        conn.execute(
            r#"INSERT INTO episodic_memories (
                session_id, content, emotion, embedding,
                stability, difficulty, retrievability,
                last_recalled_at, recall_count, status,
                created_at, updated_at, metadata
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)"#,
            params![
                item.session_id,
                item.content,
                item.emotion,
                blob,
                item.stability,
                item.difficulty,
                item.retrievability,
                item.last_recalled_at,
                item.recall_count,
                item.status,
                item.created_at,
                item.updated_at,
                item.metadata,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_active_episodic_memories(
        &self,
        session_id: &str,
    ) -> AppResult<Vec<L1EpisodicMemory>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"SELECT id, session_id, content, emotion, embedding,
                      stability, difficulty, retrievability,
                      last_recalled_at, recall_count, status,
                      created_at, updated_at, metadata
               FROM episodic_memories
               WHERE session_id = ?1 AND status = 'active'
               ORDER BY created_at DESC"#,
        )?;

        let rows = stmt.query_map(params![session_id], |row| {
            let blob_opt: Option<Vec<u8>> = row.get(4)?;
            let embedding = blob_opt.map(|b| blob_to_embedding(&b));

            Ok(L1EpisodicMemory {
                id: Some(row.get(0)?),
                session_id: row.get(1)?,
                content: row.get(2)?,
                emotion: row.get(3)?,
                embedding,
                stability: row.get(5)?,
                difficulty: row.get(6)?,
                retrievability: row.get(7)?,
                last_recalled_at: row.get(8)?,
                recall_count: row.get(9)?,
                status: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
                metadata: row.get(13)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn search_episodic_fts(
        &self,
        query: &str,
        limit: usize,
    ) -> AppResult<Vec<(L1EpisodicMemory, f64)>> {
        let conn = self.conn.lock().unwrap();
        // 清理 query 中的特殊符号防止 FTS 语法崩溃
        let sanitized = sanitize_fts_query(query);
        if sanitized.trim().is_empty() {
            return Ok(Vec::new());
        }

        let mut stmt = conn.prepare(
            r#"SELECT m.id, m.session_id, m.content, m.emotion, m.embedding,
                      m.stability, m.difficulty, m.retrievability,
                      m.last_recalled_at, m.recall_count, m.status,
                      m.created_at, m.updated_at, m.metadata,
                      bm25(episodic_fts) AS rank
               FROM episodic_fts f
               JOIN episodic_memories m ON f.rowid = m.id
               WHERE episodic_fts MATCH ?1 AND m.status = 'active'
               ORDER BY rank ASC
               LIMIT ?2"#,
        )?;

        let rows = stmt.query_map(params![sanitized, limit as i64], |row| {
            let blob_opt: Option<Vec<u8>> = row.get(4)?;
            let embedding = blob_opt.map(|b| blob_to_embedding(&b));
            let rank: f64 = row.get(14)?;

            Ok((
                L1EpisodicMemory {
                    id: Some(row.get(0)?),
                    session_id: row.get(1)?,
                    content: row.get(2)?,
                    emotion: row.get(3)?,
                    embedding,
                    stability: row.get(5)?,
                    difficulty: row.get(6)?,
                    retrievability: row.get(7)?,
                    last_recalled_at: row.get(8)?,
                    recall_count: row.get(9)?,
                    status: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                    metadata: row.get(13)?,
                },
                rank,
            ))
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    // ==========================================
    // L2: 语义事实 (Semantic Facts)
    // ==========================================

    pub fn add_semantic_fact(&self, item: &L2SemanticFact) -> AppResult<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"INSERT INTO semantic_facts (
                entity, attribute, value, confidence,
                valid_from, valid_to, source_event_id,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(entity, attribute, value) DO UPDATE SET
                confidence = excluded.confidence,
                valid_to = excluded.valid_to,
                updated_at = excluded.updated_at"#,
            params![
                item.entity,
                item.attribute,
                item.value,
                item.confidence,
                item.valid_from,
                item.valid_to,
                item.source_event_id,
                item.created_at,
                item.updated_at,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_all_semantic_facts(&self) -> AppResult<Vec<L2SemanticFact>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"SELECT id, entity, attribute, value, confidence,
                      valid_from, valid_to, source_event_id,
                      created_at, updated_at
               FROM semantic_facts
               ORDER BY updated_at DESC"#,
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(L2SemanticFact {
                id: Some(row.get(0)?),
                entity: row.get(1)?,
                attribute: row.get(2)?,
                value: row.get(3)?,
                confidence: row.get(4)?,
                valid_from: row.get(5)?,
                valid_to: row.get(6)?,
                source_event_id: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn search_semantic_fts(
        &self,
        query: &str,
        limit: usize,
    ) -> AppResult<Vec<(L2SemanticFact, f64)>> {
        let conn = self.conn.lock().unwrap();
        let sanitized = sanitize_fts_query(query);
        let mut list = Vec::new();

        if !sanitized.trim().is_empty() {
            if let Ok(mut stmt) = conn.prepare(
                r#"SELECT s.id, s.entity, s.attribute, s.value, s.confidence,
                          s.valid_from, s.valid_to, s.source_event_id,
                          s.created_at, s.updated_at,
                          bm25(semantic_fts) AS rank
                   FROM semantic_fts f
                   JOIN semantic_facts s ON f.rowid = s.id
                   WHERE semantic_fts MATCH ?1
                   ORDER BY rank ASC
                   LIMIT ?2"#,
            ) {
                if let Ok(rows) = stmt.query_map(params![sanitized, limit as i64], |row| {
                    let rank: f64 = row.get(10)?;
                    Ok((
                        L2SemanticFact {
                            id: Some(row.get(0)?),
                            entity: row.get(1)?,
                            attribute: row.get(2)?,
                            value: row.get(3)?,
                            confidence: row.get(4)?,
                            valid_from: row.get(5)?,
                            valid_to: row.get(6)?,
                            source_event_id: row.get(7)?,
                            created_at: row.get(8)?,
                            updated_at: row.get(9)?,
                        },
                        rank,
                    ))
                }) {
                    for r in rows.flatten() {
                        list.push(r);
                    }
                }
            }
        }

        // 若 FTS5 因分词器长度限制（例如 trigram 无法索引 2 字符的“用户”）未命中，执行词级/二元短语的降级召回
        if list.is_empty() {
            let mut tokens_to_try = Vec::new();
            for word in query.split_whitespace() {
                tokens_to_try.push(word.to_string());
            }
            if tokens_to_try.is_empty() || tokens_to_try.len() == 1 {
                let chars: Vec<char> = query.chars().collect();
                for w in chars.windows(2) {
                    tokens_to_try.push(w.iter().collect::<String>());
                }
            }

            for token in tokens_to_try {
                if token.chars().count() < 2 {
                    continue;
                }
                let mut fallback_stmt = conn.prepare(
                    r#"SELECT id, entity, attribute, value, confidence,
                              valid_from, valid_to, source_event_id,
                              created_at, updated_at
                       FROM semantic_facts
                       WHERE entity LIKE '%' || ?1 || '%'
                          OR attribute LIKE '%' || ?1 || '%'
                          OR value LIKE '%' || ?1 || '%'
                       LIMIT ?2"#,
                )?;
                let rows = fallback_stmt.query_map(params![token, limit as i64], |row| {
                    Ok((
                        L2SemanticFact {
                            id: Some(row.get(0)?),
                            entity: row.get(1)?,
                            attribute: row.get(2)?,
                            value: row.get(3)?,
                            confidence: row.get(4)?,
                            valid_from: row.get(5)?,
                            valid_to: row.get(6)?,
                            source_event_id: row.get(7)?,
                            created_at: row.get(8)?,
                            updated_at: row.get(9)?,
                        },
                        0.5,
                    ))
                })?;
                for r in rows.flatten() {
                    if !list.iter().any(|(x, _)| x.id == r.0.id) {
                        list.push(r);
                    }
                }
                if list.len() >= limit {
                    break;
                }
            }
        }

        Ok(list)
    }

    // ==========================================
    // L3: 叙事记忆 (Narratives)
    // ==========================================

    pub fn upsert_narrative(&self, item: &L3Narrative) -> AppResult<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"INSERT INTO narratives (
                period_type, period_key, summary, reflection,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(period_type, period_key) DO UPDATE SET
                summary = excluded.summary,
                reflection = excluded.reflection,
                updated_at = excluded.updated_at"#,
            params![
                item.period_type,
                item.period_key,
                item.summary,
                item.reflection,
                item.created_at,
                item.updated_at,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_narratives(
        &self,
        period_type: Option<&str>,
        limit: usize,
    ) -> AppResult<Vec<L3Narrative>> {
        let conn = self.conn.lock().unwrap();
        let (sql, params_vec): (&str, Vec<rusqlite::types::Value>) = match period_type {
            Some(pt) => (
                "SELECT id, period_type, period_key, summary, reflection, created_at, updated_at
                 FROM narratives WHERE period_type = ?1 ORDER BY period_key DESC LIMIT ?2",
                vec![
                    rusqlite::types::Value::Text(pt.to_string()),
                    rusqlite::types::Value::Integer(limit as i64),
                ],
            ),
            None => (
                "SELECT id, period_type, period_key, summary, reflection, created_at, updated_at
                 FROM narratives ORDER BY period_key DESC LIMIT ?1",
                vec![rusqlite::types::Value::Integer(limit as i64)],
            ),
        };

        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params_vec), |row| {
            Ok(L3Narrative {
                id: Some(row.get(0)?),
                period_type: row.get(1)?,
                period_key: row.get(2)?,
                summary: row.get(3)?,
                reflection: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    // ==========================================
    // L4: 核心画像 (Profiles)
    // ==========================================

    pub fn set_profile(&self, category: &str, key: &str, value: &str) -> AppResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp_millis();
        conn.execute(
            r#"INSERT INTO profiles (category, key, value, created_at, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?4)
               ON CONFLICT(key) DO UPDATE SET
                   value = excluded.value,
                   updated_at = excluded.updated_at"#,
            params![category, key, value, now],
        )?;
        Ok(())
    }

    pub fn get_profile_value(&self, key: &str) -> AppResult<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM profiles WHERE key = ?1")?;
        let res = stmt
            .query_row(params![key], |row| row.get(0))
            .optional()?;
        Ok(res)
    }

    pub fn get_profiles(&self, category: Option<&str>) -> AppResult<Vec<L4Profile>> {
        let conn = self.conn.lock().unwrap();
        let (sql, params_vec): (&str, Vec<rusqlite::types::Value>) = match category {
            Some(cat) => (
                "SELECT id, category, key, value, created_at, updated_at
                 FROM profiles WHERE category = ?1 ORDER BY id ASC",
                vec![rusqlite::types::Value::Text(cat.to_string())],
            ),
            None => (
                "SELECT id, category, key, value, created_at, updated_at
                 FROM profiles ORDER BY id ASC",
                vec![],
            ),
        };

        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params_vec), |row| {
            Ok(L4Profile {
                id: Some(row.get(0)?),
                category: row.get(1)?,
                key: row.get(2)?,
                value: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }
}

/// 净化 FTS5 查询字符串，过滤单双引号及非安全字符，并对中文长词拆分 n-gram 以便 FTS 倒排索引匹配
fn sanitize_fts_query(input: &str) -> String {
    let mut cleaned = String::new();
    for c in input.chars() {
        if c.is_alphanumeric() || c.is_whitespace() || ('\u{4e00}'..='\u{9fa5}').contains(&c) {
            cleaned.push(c);
        } else {
            cleaned.push(' ');
        }
    }
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    if words.is_empty() {
        return String::new();
    }

    let mut tokens: Vec<String> = Vec::new();
    for word in words {
        let chars: Vec<char> = word.chars().collect();
        if chars.len() <= 3 {
            tokens.push(format!("\"{}\"", word));
        } else {
            let is_cjk = chars.iter().any(|c| ('\u{4e00}'..='\u{9fa5}').contains(c));
            if is_cjk {
                // 提取 3-grams (针对 trigram tokenizer)
                for w in chars.windows(3) {
                    let s: String = w.iter().collect();
                    tokens.push(format!("\"{}\"", s));
                }
                // 提取 2-grams
                for w in chars.windows(2) {
                    let s: String = w.iter().collect();
                    tokens.push(format!("\"{}\"", s));
                }
            } else {
                tokens.push(format!("\"{}\"", word));
            }
        }
    }

    if tokens.is_empty() {
        return String::new();
    }

    let mut seen = std::collections::HashSet::new();
    tokens.retain(|t| seen.insert(t.clone()));
    tokens.truncate(16);

    tokens.join(" OR ")
}
