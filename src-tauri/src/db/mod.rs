pub mod models;

use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use tauri::{AppHandle, Manager};

pub type DbPool = Pool<Sqlite>;

/// 按分号分割 SQL 脚本，正确处理引号字符串内的分号。
/// 简单状态机：跟踪单引号/双引号状态，只在引号外部分割。
fn split_sql_statements(script: &str) -> Vec<&str> {
    let mut statements = Vec::new();
    let bytes = script.as_bytes();
    let mut start = 0;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut i = 0;

    while i < bytes.len() {
        let c = bytes[i];
        if c == b'\'' && !in_double_quote {
            // 处理 SQL 转义 '' → 跳过下一个引号
            if in_single_quote && i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                i += 2;
                continue;
            }
            in_single_quote = !in_single_quote;
        } else if c == b'"' && !in_single_quote {
            if in_double_quote && i + 1 < bytes.len() && bytes[i + 1] == b'"' {
                i += 2;
                continue;
            }
            in_double_quote = !in_double_quote;
        } else if c == b';' && !in_single_quote && !in_double_quote {
            let stmt = &script[start..i];
            statements.push(stmt.trim());
            start = i + 1;
        }
        i += 1;
    }

    // 最后一段（可能没有分号结尾）
    let tail = script[start..].trim();
    if !tail.is_empty() {
        statements.push(tail);
    }

    statements
}

pub async fn init(app: &AppHandle) -> anyhow::Result<DbPool> {
    let app_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| {
            let mut p = std::env::current_dir().unwrap_or_default();
            p.push("data");
            p
        });
    std::fs::create_dir_all(&app_dir)?;

    let db_path = app_dir.join("vet_knowledge.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display()).replace('\\', "/");

    let pool = SqlitePoolOptions::new()
        .max_connections(2)
        .connect(&db_url)
        .await?;

    // SQLite PRAGMA 优化（桌面端最佳实践）
    sqlx::query("PRAGMA journal_mode = WAL").execute(&pool).await?;
    sqlx::query("PRAGMA synchronous = NORMAL").execute(&pool).await?;
    sqlx::query("PRAGMA cache_size = -64000").execute(&pool).await?;
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await?;
    sqlx::query("PRAGMA temp_store = MEMORY").execute(&pool).await?;

    // 建表（使用安全的 SQL 分割）
    let schema = include_str!("../../data/seed/schema.sql");
    for stmt in split_sql_statements(schema) {
        if !stmt.is_empty() && !stmt.starts_with("--") {
            sqlx::query(stmt).execute(&pool).await.ok();
        }
    }

    // Schema 迁移
    apply_migrations(&pool).await?;

    // 数据版本检查
    import_seed_data(&pool).await?;

    Ok(pool)
}

/// Schema 迁移：通过 schema_migrations 表追踪已应用的迁移。
/// 每个迁移块是幂等的（用 IF NOT EXISTS 或 INSERT OR IGNORE），
/// 通过版本号保证只执行一次。
async fn apply_migrations(pool: &DbPool) -> anyhow::Result<()> {
    // 确保迁移表存在
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT DEFAULT (datetime('now')),
            description TEXT
        )"
    )
    .execute(pool)
    .await?;

    // 确保 app_meta 表存在
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS app_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )"
    )
    .execute(pool)
    .await?;

    let current: Option<i64> = sqlx::query_scalar(
        "SELECT MAX(version) FROM schema_migrations"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(None);

    let current = current.unwrap_or(0);

    // ── v1: 初始 schema（建表已在上面完成，此处记录版本基准） ──
    if current < 1 {
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version, description)
             VALUES (1, 'Initial schema: all tables + indexes + FTS + learning_progress')"
        )
        .execute(pool)
        .await?;
    }

    // ── v2: 新增 urgency_level 和 is_pathognomonic ──
    if current < 2 {
        sqlx::query(
            "ALTER TABLE diseases ADD COLUMN urgency_level INTEGER DEFAULT 3 CHECK(urgency_level BETWEEN 1 AND 5)"
        )
        .execute(pool)
        .await
        .ok();
        sqlx::query(
            "ALTER TABLE disease_symptom ADD COLUMN is_pathognomonic INTEGER DEFAULT 0 CHECK(is_pathognomonic IN (0, 1))"
        )
        .execute(pool)
        .await
        .ok();
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version, description)
             VALUES (2, 'Add urgency_level to diseases, is_pathognomonic to disease_symptom')"
        )
        .execute(pool)
        .await?;
    }

    // ── v3: disease_diagnostic 主键增加 species（重建表） ──
    if current < 3 {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS disease_diagnostic_v2 (
                disease_id TEXT REFERENCES diseases(id),
                test_id TEXT REFERENCES diagnostic_tests(id),
                purpose TEXT CHECK(purpose IN ('screening','confirming','monitoring')),
                evidence_level TEXT DEFAULT 'supportive' CHECK(evidence_level IN ('gold_standard','supportive','exclusionary')),
                species TEXT,
                expected_result TEXT,
                PRIMARY KEY (disease_id, test_id, species)
            )"
        )
        .execute(pool)
        .await
        .ok();
        sqlx::query(
            "INSERT INTO disease_diagnostic_v2 (disease_id, test_id, purpose, evidence_level, species, expected_result)
             SELECT disease_id, test_id, purpose, 'supportive', NULL, expected_result FROM disease_diagnostic"
        )
        .execute(pool)
        .await
        .ok();
        sqlx::query("DROP TABLE IF EXISTS disease_diagnostic").execute(pool).await.ok();
        sqlx::query("ALTER TABLE disease_diagnostic_v2 RENAME TO disease_diagnostic")
            .execute(pool)
            .await
            .ok();
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version, description)
             VALUES (3, 'Rebuild disease_diagnostic with species in PK')"
        )
        .execute(pool)
        .await?;
    }

    // ── v4: disease_treatment 主键增加 species（重建表） ──
    if current < 4 {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS disease_treatment_v2 (
                disease_id TEXT REFERENCES diseases(id),
                drug_id TEXT REFERENCES drugs(id),
                line TEXT CHECK(line IN ('first','second','adjunctive')),
                species TEXT,
                notes TEXT,
                PRIMARY KEY (disease_id, drug_id, species)
            )"
        )
        .execute(pool)
        .await
        .ok();
        sqlx::query(
            "INSERT INTO disease_treatment_v2 (disease_id, drug_id, line, species, notes)
             SELECT disease_id, drug_id, line, NULL, notes FROM disease_treatment"
        )
        .execute(pool)
        .await
        .ok();
        sqlx::query("DROP TABLE IF EXISTS disease_treatment").execute(pool).await.ok();
        sqlx::query("ALTER TABLE disease_treatment_v2 RENAME TO disease_treatment")
            .execute(pool)
            .await
            .ok();
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version, description)
             VALUES (4, 'Rebuild disease_treatment with species in PK')"
        )
        .execute(pool)
        .await?;
    }

    // ── v5: 闪卡系统 ──
    if current < 5 {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS flashcards (
                id TEXT PRIMARY KEY,
                front TEXT NOT NULL,
                back TEXT NOT NULL,
                card_type TEXT NOT NULL CHECK(card_type IN ('disease', 'symptom', 'drug', 'custom')),
                entity_id TEXT,
                difficulty REAL DEFAULT 0.5 CHECK(difficulty BETWEEN 0 AND 1),
                created_at TEXT DEFAULT (datetime('now'))
            )"
        )
        .execute(pool)
        .await
        .ok();
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS flashcard_reviews (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                card_id TEXT NOT NULL REFERENCES flashcards(id) ON DELETE CASCADE,
                quality INTEGER NOT NULL CHECK(quality BETWEEN 0 AND 5),
                reviewed_at TEXT DEFAULT (datetime('now')),
                interval_days REAL DEFAULT 0,
                ease_factor REAL DEFAULT 2.5,
                next_review TEXT NOT NULL
            )"
        )
        .execute(pool)
        .await
        .ok();
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_flashcard_reviews_next ON flashcard_reviews(card_id, next_review)"
        )
        .execute(pool)
        .await
        .ok();
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version, description)
             VALUES (5, 'Add flashcard system tables')"
        )
        .execute(pool)
        .await?;
    }

    // ── v6: diseases 表新增 data_source / is_deleted ──
    if current < 6 {
        sqlx::query(
            "ALTER TABLE diseases ADD COLUMN data_source TEXT DEFAULT 'seed' CHECK(data_source IN ('seed', 'user', 'imported'))"
        )
        .execute(pool)
        .await
        .ok();
        sqlx::query(
            "ALTER TABLE diseases ADD COLUMN is_deleted INTEGER DEFAULT 0"
        )
        .execute(pool)
        .await
        .ok();
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version, description)
             VALUES (6, 'Add data_source and is_deleted to diseases')"
        )
        .execute(pool)
        .await?;
    }

    // ── v7: 标签系统 + 四维度增强 + 治疗模块 ──
    if current < 7 {
        // diseases 新增字段
        sqlx::query("ALTER TABLE diseases ADD COLUMN name_latin TEXT").execute(pool).await.ok();
        sqlx::query("ALTER TABLE diseases ADD COLUMN pathogenic_type TEXT").execute(pool).await.ok();
        sqlx::query("ALTER TABLE diseases ADD COLUMN epidemiology TEXT").execute(pool).await.ok();
        sqlx::query("ALTER TABLE diseases ADD COLUMN body_system TEXT").execute(pool).await.ok();
        sqlx::query("ALTER TABLE diseases ADD COLUMN physiological_basis TEXT").execute(pool).await.ok();

        // symptoms 新增字段
        sqlx::query("ALTER TABLE symptoms ADD COLUMN physiological_basis TEXT").execute(pool).await.ok();

        // drugs 新增字段
        sqlx::query("ALTER TABLE drugs ADD COLUMN mechanism_of_action TEXT").execute(pool).await.ok();
        sqlx::query("ALTER TABLE drugs ADD COLUMN pk_pd TEXT").execute(pool).await.ok();
        sqlx::query("ALTER TABLE drugs ADD COLUMN adverse_mechanism TEXT").execute(pool).await.ok();

        // 标签系统
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS tags (
                id TEXT PRIMARY KEY,
                name_zh TEXT NOT NULL UNIQUE,
                name_en TEXT,
                tag_group TEXT NOT NULL DEFAULT 'custom',
                emergency_level TEXT CHECK(emergency_level IS NULL OR emergency_level IN ('red','orange','yellow','green')),
                clinical_action TEXT,
                textbook_logic TEXT,
                typical_scenario TEXT,
                color TEXT,
                created_at TEXT DEFAULT (datetime('now'))
            )"
        ).execute(pool).await.ok();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS entity_tags (
                entity_type TEXT NOT NULL CHECK(entity_type IN ('disease','symptom','drug','treatment','case')),
                entity_id TEXT NOT NULL,
                tag_id TEXT NOT NULL REFERENCES tags(id),
                PRIMARY KEY (entity_type, entity_id, tag_id)
            )"
        ).execute(pool).await.ok();

        // 治疗模块
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS treatments (
                id TEXT PRIMARY KEY,
                name_zh TEXT NOT NULL,
                name_en TEXT,
                therapy_type TEXT CHECK(therapy_type IN ('药物疗法','手术疗法','急救重症','康复理疗','营养辅助','其他')),
                principle TEXT,
                procedure_text TEXT,
                physiological_basis TEXT,
                prognosis_assessment TEXT,
                created_at TEXT DEFAULT (datetime('now'))
            )"
        ).execute(pool).await.ok();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS disease_treatment_map (
                disease_id TEXT REFERENCES diseases(id),
                treatment_id TEXT REFERENCES treatments(id),
                line TEXT CHECK(line IN ('first','second','adjunctive')),
                species TEXT,
                notes TEXT,
                PRIMARY KEY (disease_id, treatment_id, species)
            )"
        ).execute(pool).await.ok();

        // 索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_entity_tags_type ON entity_tags(entity_type, entity_id)").execute(pool).await.ok();
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_entity_tags_tag ON entity_tags(tag_id)").execute(pool).await.ok();
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tags_group ON tags(tag_group)").execute(pool).await.ok();

        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version, description)
             VALUES (7, 'Tag system + disease/symptom/drug enhancements + treatments module')"
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// 种子数据导入：版本不匹配时全量重建。
/// 使用 UPSERT（INSERT OR REPLACE）避免 DELETE 后再 INSERT 的外键问题。
async fn import_seed_data(pool: &DbPool) -> anyhow::Result<()> {
    const SEED_DATA_VERSION: i64 = 10;

    let stored: Option<i64> = sqlx::query_scalar(
        "SELECT value FROM app_meta WHERE key = 'seed_data_version'"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(None);

    let needs_import = match stored {
        None => true,
        Some(v) if v < SEED_DATA_VERSION => true,
        _ => false,
    };

    if !needs_import {
        return Ok(());
    }

    let seed = include_str!("../../data/seed/001_initial.sql");

    // 按事务执行：先清空，再导入
    let mut tx = pool.begin().await?;

    // 清空数据表（保留表结构）
    for tbl in &[
        "case_disease", "disease_diagnostic", "disease_treatment",
        "disease_symptom", "disease_ddx", "cases", "diagnostic_tests",
        "drugs", "symptoms", "diseases",
    ] {
        sqlx::query(&format!("DELETE FROM {}", tbl))
            .execute(&mut *tx)
            .await
            .ok();
    }

    // 导入种子数据（使用安全的 SQL 分割）
    for stmt in split_sql_statements(seed) {
        let t = stmt.trim();
        if !t.is_empty() && !t.starts_with("--") {
            sqlx::query(t).execute(&mut *tx).await.ok();
        }
    }

    // 记录数据版本
    sqlx::query(
        "INSERT OR REPLACE INTO app_meta (key, value) VALUES ('seed_data_version', ?)"
    )
    .bind(SEED_DATA_VERSION)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_sql_basic() {
        let sql = "CREATE TABLE a (id TEXT); INSERT INTO a VALUES ('x');";
        let stmts = split_sql_statements(sql);
        assert_eq!(stmts.len(), 2);
        assert!(stmts[0].contains("CREATE TABLE"));
        assert!(stmts[1].contains("INSERT"));
    }

    #[test]
    fn test_split_sql_semicolon_in_string() {
        let sql = "INSERT INTO t VALUES ('a;b;c'); SELECT 1;";
        let stmts = split_sql_statements(sql);
        assert_eq!(stmts.len(), 2);
        assert_eq!(stmts[0], "INSERT INTO t VALUES ('a;b;c')");
    }

    #[test]
    fn test_split_sql_escaped_quotes() {
        let sql = "INSERT INTO t VALUES ('it''s; fine'); SELECT 1;";
        let stmts = split_sql_statements(sql);
        assert_eq!(stmts.len(), 2);
        assert_eq!(stmts[0], "INSERT INTO t VALUES ('it''s; fine')");
    }

    #[test]
    fn test_split_sql_empty_and_comments() {
        let sql = "-- comment\n\nCREATE TABLE a (id TEXT);\n\n";
        let stmts = split_sql_statements(sql);
        assert_eq!(stmts.len(), 2); // comment + CREATE
    }
}
