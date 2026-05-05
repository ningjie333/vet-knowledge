pub mod models;

use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use tauri::{AppHandle, Manager};

pub type DbPool = Pool<Sqlite>;

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

    // 建表
    let schema = include_str!("../../data/seed/schema.sql");
    for stmt in schema.split(';') {
        let t = stmt.trim();
        if !t.is_empty() {
            sqlx::query(t).execute(&pool).await.ok();
        }
    }

    // 确保 app_meta 表存在（用于数据版本追踪）
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS app_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )"
    )
    .execute(&pool)
    .await?;

    // 确保关键表的列完整（兼容旧数据库）
    ensure_columns(&pool).await;

    // Schema 迁移：检查并执行未应用的迁移
    apply_migrations(&pool).await?;

    // 数据版本检查：版本不匹配时重新导入种子数据
    const SEED_DATA_VERSION: i64 = 8;
    let stored_version: Option<i64> = sqlx::query_scalar(
        "SELECT value FROM app_meta WHERE key = 'seed_data_version'"
    )
    .fetch_one(&pool)
    .await
    .unwrap_or(None);

    let needs_import = match stored_version {
        None => true,
        Some(v) if v < SEED_DATA_VERSION => true,
        _ => false,
    };

    if needs_import {
        // 关闭外键约束，避免 DELETE 和 INSERT 顺序问题
        sqlx::query("PRAGMA foreign_keys = OFF").execute(&pool).await.ok();

        // 清空所有数据表（保留表结构）
        // 注意：data_source 隔离需要所有表都有该列，目前仅 diseases 表有
        // 完整隔离在 v8 迁移中实现，当前版本先全量清除保证正确性
        for tbl in &[
            "case_disease", "disease_diagnostic", "disease_treatment",
            "disease_symptom", "disease_ddx", "cases", "diagnostic_tests",
            "drugs", "symptoms", "diseases",
        ] {
            sqlx::query(&format!("DELETE FROM {}", tbl)).execute(&pool).await.ok();
        }

        // 导入种子数据
        let seed = include_str!("../../data/seed/001_initial.sql");
        for stmt in seed.split(';') {
            let t = stmt.trim();
            if !t.is_empty() && !t.starts_with("--") {
                sqlx::query(t).execute(&pool).await.ok();
            }
        }

        // 恢复外键约束
        sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.ok();

        // 记录数据版本
        sqlx::query(
            "INSERT OR REPLACE INTO app_meta (key, value) VALUES ('seed_data_version', ?)"
        )
        .bind(SEED_DATA_VERSION)
        .execute(&pool)
        .await?;
    }

    Ok(pool)
}

/// 确保关键表包含所有需要的列（幂等，兼容旧版本数据库）
/// SQLite 不支持 ADD COLUMN IF NOT EXISTS，用 ok() 忽略"列已存在"错误
async fn ensure_columns(pool: &DbPool) {
    // diseases 表可能缺少 urgency_level（v2 迁移添加）
    sqlx::query("ALTER TABLE diseases ADD COLUMN urgency_level INTEGER DEFAULT 3 CHECK(urgency_level BETWEEN 1 AND 5)")
        .execute(pool).await.ok();
    // diseases 表可能缺少 data_source（v7 迁移添加）
    sqlx::query("ALTER TABLE diseases ADD COLUMN data_source TEXT DEFAULT 'seed' CHECK(data_source IN ('seed', 'user', 'imported'))")
        .execute(pool).await.ok();
    // diseases 表可能缺少 is_deleted（v7 迁移添加）
    sqlx::query("ALTER TABLE diseases ADD COLUMN is_deleted INTEGER DEFAULT 0")
        .execute(pool).await.ok();
    // disease_symptom 表可能缺少 is_pathognomonic
    sqlx::query("ALTER TABLE disease_symptom ADD COLUMN is_pathognomonic INTEGER DEFAULT 0 CHECK(is_pathognomonic IN (0, 1))")
        .execute(pool).await.ok();
}

/// 检查 schema_migrations 表，执行未应用的迁移
/// 注意：SQLite ALTER TABLE 有限制，主键变更需要重建表
async fn apply_migrations(pool: &DbPool) -> anyhow::Result<()> {
    let current_version: Option<i64> = sqlx::query_scalar(
        "SELECT MAX(version) FROM schema_migrations"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(None);

    let current = current_version.unwrap_or(0);

    // v2: 新增 urgency_level 和 is_pathognomonic 字段
    if current < 2 {
        sqlx::query("ALTER TABLE diseases ADD COLUMN urgency_level INTEGER DEFAULT 3 CHECK(urgency_level BETWEEN 1 AND 5)")
            .execute(pool).await.ok();
        sqlx::query("ALTER TABLE disease_symptom ADD COLUMN is_pathognomonic INTEGER DEFAULT 0 CHECK(is_pathognomonic IN (0, 1))")
            .execute(pool).await.ok();
        sqlx::query("INSERT OR IGNORE INTO schema_migrations (version, description) VALUES (2, 'Add urgency_level and is_pathognomonic')")
            .execute(pool).await?;
    }

    // v3: disease_diagnostic 主键增加 species（重建表）
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
        ).execute(pool).await.ok();
        // 迁移旧数据（species 设为 NULL 表示通用）
        sqlx::query(
            "INSERT INTO disease_diagnostic_v2 (disease_id, test_id, purpose, evidence_level, species, expected_result)
             SELECT disease_id, test_id, purpose, 'supportive', NULL, expected_result FROM disease_diagnostic"
        ).execute(pool).await.ok();
        sqlx::query("DROP TABLE IF EXISTS disease_diagnostic").execute(pool).await.ok();
        sqlx::query("ALTER TABLE disease_diagnostic_v2 RENAME TO disease_diagnostic").execute(pool).await.ok();
        sqlx::query("INSERT OR IGNORE INTO schema_migrations (version, description) VALUES (3, 'Rebuild disease_diagnostic with species and evidence_level')")
            .execute(pool).await?;
    }

    // v5: 闪卡系统 — 闪卡表和复习记录表
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
        ).execute(pool).await.ok();
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
        ).execute(pool).await.ok();
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_flashcard_reviews_next ON flashcard_reviews(card_id, next_review)"
        ).execute(pool).await.ok();
        sqlx::query("INSERT OR IGNORE INTO schema_migrations (version, description) VALUES (5, 'Add flashcard system tables')")
            .execute(pool).await?;
    }

    // v4: disease_treatment 主键增加 species（重建表）
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
        ).execute(pool).await.ok();
        sqlx::query(
            "INSERT INTO disease_treatment_v2 (disease_id, drug_id, line, species, notes)
             SELECT disease_id, drug_id, line, NULL, notes FROM disease_treatment"
        ).execute(pool).await.ok();
        sqlx::query("DROP TABLE IF EXISTS disease_treatment").execute(pool).await.ok();
        sqlx::query("ALTER TABLE disease_treatment_v2 RENAME TO disease_treatment").execute(pool).await.ok();
        sqlx::query("INSERT OR IGNORE INTO schema_migrations (version, description) VALUES (4, 'Rebuild disease_treatment with species')")
            .execute(pool).await?;
    }

    Ok(())
}
