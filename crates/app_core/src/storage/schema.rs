//! SQLite 数据库架构定义 (L0 ~ L4 五层存储架构与 FTS5 虚拟表)

pub const INIT_SQL_TRIGRAM: &str = r#"
-- L0 工作记忆表
CREATE TABLE IF NOT EXISTS conversations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    metadata TEXT
);
CREATE INDEX IF NOT EXISTS idx_conversations_session ON conversations(session_id, created_at);

-- L1 情景记忆表
CREATE TABLE IF NOT EXISTS episodic_memories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL DEFAULT 'default',
    content TEXT NOT NULL,
    emotion TEXT,
    embedding BLOB,
    stability REAL NOT NULL DEFAULT 1.0,
    difficulty REAL NOT NULL DEFAULT 5.0,
    retrievability REAL NOT NULL DEFAULT 1.0,
    last_recalled_at INTEGER NOT NULL,
    recall_count INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'active',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    metadata TEXT
);
CREATE INDEX IF NOT EXISTS idx_episodic_session ON episodic_memories(session_id, status);
CREATE INDEX IF NOT EXISTS idx_episodic_created ON episodic_memories(created_at);

-- L1 FTS5 全文索引 (使用 trigram 分词器支持中文子串检索)
CREATE VIRTUAL TABLE IF NOT EXISTS episodic_fts USING fts5(
    content,
    content='episodic_memories',
    content_rowid='id',
    tokenize='trigram'
);

-- L1 触发器
CREATE TRIGGER IF NOT EXISTS episodic_ai AFTER INSERT ON episodic_memories BEGIN
    INSERT INTO episodic_fts(rowid, content) VALUES (new.id, new.content);
END;
CREATE TRIGGER IF NOT EXISTS episodic_au AFTER UPDATE OF content ON episodic_memories BEGIN
    INSERT INTO episodic_fts(episodic_fts, rowid, content) VALUES('delete', old.id, old.content);
    INSERT INTO episodic_fts(rowid, content) VALUES (new.id, new.content);
END;
CREATE TRIGGER IF NOT EXISTS episodic_ad AFTER DELETE ON episodic_memories BEGIN
    INSERT INTO episodic_fts(episodic_fts, rowid, content) VALUES('delete', old.id, old.content);
END;

-- L2 语义事实表
CREATE TABLE IF NOT EXISTS semantic_facts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity TEXT NOT NULL,
    attribute TEXT NOT NULL,
    value TEXT NOT NULL,
    confidence REAL NOT NULL DEFAULT 1.0,
    valid_from INTEGER,
    valid_to INTEGER,
    source_event_id INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(entity, attribute, value)
);
CREATE INDEX IF NOT EXISTS idx_semantic_entity ON semantic_facts(entity);

-- L2 FTS5 全文索引
CREATE VIRTUAL TABLE IF NOT EXISTS semantic_fts USING fts5(
    entity,
    attribute,
    value,
    content='semantic_facts',
    content_rowid='id',
    tokenize='trigram'
);

-- L2 触发器
CREATE TRIGGER IF NOT EXISTS semantic_ai AFTER INSERT ON semantic_facts BEGIN
    INSERT INTO semantic_fts(rowid, entity, attribute, value) VALUES (new.id, new.entity, new.attribute, new.value);
END;
CREATE TRIGGER IF NOT EXISTS semantic_au AFTER UPDATE OF entity, attribute, value ON semantic_facts BEGIN
    INSERT INTO semantic_fts(semantic_fts, rowid, entity, attribute, value) VALUES('delete', old.id, old.entity, old.attribute, old.value);
    INSERT INTO semantic_fts(rowid, entity, attribute, value) VALUES (new.id, new.entity, new.attribute, new.value);
END;
CREATE TRIGGER IF NOT EXISTS semantic_ad AFTER DELETE ON semantic_facts BEGIN
    INSERT INTO semantic_fts(semantic_fts, rowid, entity, attribute, value) VALUES('delete', old.id, old.entity, old.attribute, old.value);
END;

-- L3 叙事记忆表
CREATE TABLE IF NOT EXISTS narratives (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    period_type TEXT NOT NULL,
    period_key TEXT NOT NULL,
    summary TEXT NOT NULL,
    reflection TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(period_type, period_key)
);

-- L4 核心画像表
CREATE TABLE IF NOT EXISTS profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    category TEXT NOT NULL,
    key TEXT NOT NULL UNIQUE,
    value TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- 架构元数据表
CREATE TABLE IF NOT EXISTS schema_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
"#;

pub const INIT_SQL_FALLBACK: &str = r#"
-- L0 工作记忆表
CREATE TABLE IF NOT EXISTS conversations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    metadata TEXT
);
CREATE INDEX IF NOT EXISTS idx_conversations_session ON conversations(session_id, created_at);

-- L1 情景记忆表
CREATE TABLE IF NOT EXISTS episodic_memories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL DEFAULT 'default',
    content TEXT NOT NULL,
    emotion TEXT,
    embedding BLOB,
    stability REAL NOT NULL DEFAULT 1.0,
    difficulty REAL NOT NULL DEFAULT 5.0,
    retrievability REAL NOT NULL DEFAULT 1.0,
    last_recalled_at INTEGER NOT NULL,
    recall_count INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'active',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    metadata TEXT
);
CREATE INDEX IF NOT EXISTS idx_episodic_session ON episodic_memories(session_id, status);
CREATE INDEX IF NOT EXISTS idx_episodic_created ON episodic_memories(created_at);

-- L1 FTS5 全文索引 (标准分词器)
CREATE VIRTUAL TABLE IF NOT EXISTS episodic_fts USING fts5(
    content,
    content='episodic_memories',
    content_rowid='id'
);

-- L1 触发器
CREATE TRIGGER IF NOT EXISTS episodic_ai AFTER INSERT ON episodic_memories BEGIN
    INSERT INTO episodic_fts(rowid, content) VALUES (new.id, new.content);
END;
CREATE TRIGGER IF NOT EXISTS episodic_au AFTER UPDATE OF content ON episodic_memories BEGIN
    INSERT INTO episodic_fts(episodic_fts, rowid, content) VALUES('delete', old.id, old.content);
    INSERT INTO episodic_fts(rowid, content) VALUES (new.id, new.content);
END;
CREATE TRIGGER IF NOT EXISTS episodic_ad AFTER DELETE ON episodic_memories BEGIN
    INSERT INTO episodic_fts(episodic_fts, rowid, content) VALUES('delete', old.id, old.content);
END;

-- L2 语义事实表
CREATE TABLE IF NOT EXISTS semantic_facts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity TEXT NOT NULL,
    attribute TEXT NOT NULL,
    value TEXT NOT NULL,
    confidence REAL NOT NULL DEFAULT 1.0,
    valid_from INTEGER,
    valid_to INTEGER,
    source_event_id INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(entity, attribute, value)
);
CREATE INDEX IF NOT EXISTS idx_semantic_entity ON semantic_facts(entity);

-- L2 FTS5 全文索引
CREATE VIRTUAL TABLE IF NOT EXISTS semantic_fts USING fts5(
    entity,
    attribute,
    value,
    content='semantic_facts',
    content_rowid='id'
);

-- L2 触发器
CREATE TRIGGER IF NOT EXISTS semantic_ai AFTER INSERT ON semantic_facts BEGIN
    INSERT INTO semantic_fts(rowid, entity, attribute, value) VALUES (new.id, new.entity, new.attribute, new.value);
END;
CREATE TRIGGER IF NOT EXISTS semantic_au AFTER UPDATE OF entity, attribute, value ON semantic_facts BEGIN
    INSERT INTO semantic_fts(semantic_fts, rowid, entity, attribute, value) VALUES('delete', old.id, old.entity, old.attribute, old.value);
    INSERT INTO semantic_fts(rowid, entity, attribute, value) VALUES (new.id, new.entity, new.attribute, new.value);
END;
CREATE TRIGGER IF NOT EXISTS semantic_ad AFTER DELETE ON semantic_facts BEGIN
    INSERT INTO semantic_fts(semantic_fts, rowid, entity, attribute, value) VALUES('delete', old.id, old.entity, old.attribute, old.value);
END;

-- L3 叙事记忆表
CREATE TABLE IF NOT EXISTS narratives (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    period_type TEXT NOT NULL,
    period_key TEXT NOT NULL,
    summary TEXT NOT NULL,
    reflection TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(period_type, period_key)
);

-- L4 核心画像表
CREATE TABLE IF NOT EXISTS profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    category TEXT NOT NULL,
    key TEXT NOT NULL UNIQUE,
    value TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- 架构元数据表
CREATE TABLE IF NOT EXISTS schema_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
"#;
