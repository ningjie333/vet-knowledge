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

/// 去掉 SQL 脚本里所有 `--` 单行注释。
/// 行为：行首 `--` 整行删除（inline `--` 不影响，SQLite 自己处理）。
fn strip_sql_comments(script: &str) -> String {
    script
        .lines()
        .filter(|l| !l.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 容忍错误的 SQL 执行（仅用于无法用 `IF NOT EXISTS` 的场景，如 ALTER TABLE ADD COLUMN / FTS5 操作）。
///
/// 失败原因记录到 stderr 但不阻断迁移流程。依据 E-08 规范：
/// 不允许 `.ok()` 完全静默吞掉错误，必须至少有日志输出。
///
/// 对于幂等的 `CREATE/DROP/INDEX IF NOT EXISTS` 语句，应该用 `?` 让错误传播，
/// 而不是用本函数。
async fn execute_tolerant(pool: &DbPool, sql: &str, ctx: &str) {
    if let Err(e) = sqlx::query(sql).execute(pool).await {
        eprintln!("[migration {}] SQL failed: {} | error: {}", ctx, sql, e);
    }
}

/// 当前种子数据版本。递增此值时，应用启动时会自动重新导入种子数据。
const SEED_DATA_VERSION: i64 = 18;

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
    // 关键：先 strip_sql_comments 把 `--` 单行注释剥掉，
    // 否则 split_sql_statements 输出的第一个 statement 会以 `--` 开头被跳过 → 表不建。
    let schema = strip_sql_comments(include_str!("../../data/seed/schema.sql"));
    for stmt in split_sql_statements(&schema) {
        let t = stmt.trim();
        if !t.is_empty() {
            sqlx::query(t).execute(&pool).await?;
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

    // 对全新数据库，schema.sql 已经是最新结构，不需要再回放历史迁移。
    // 直接写入当前迁移基线，避免旧迁移在新 schema 上重复重建表。
    if current == 0 {
        for (version, description) in [
            (1, "Initial schema: all tables + indexes + learning_progress"),
            (2, "Add urgency_level to diseases, is_pathognomonic to disease_symptom"),
            (3, "Rebuild disease_diagnostic with species in PK"),
            (4, "Rebuild disease_treatment with species in PK"),
            (5, "Add flashcard system tables"),
            (6, "Add data_source and is_deleted to diseases"),
            (7, "Tag system + disease/symptom/drug enhancements + treatments module"),
            (8, "Add cases_fts for full-text search on cases (deprecated in v10)"),
            (9, "Rebuild tags table without UNIQUE(name_zh)"),
            (10, "Drop unused FTS5 virtual tables (diseases_fts, symptoms_fts, drugs_fts, cases_fts)"),
        ] {
            sqlx::query(
                "INSERT OR IGNORE INTO schema_migrations (version, description)
                 VALUES (?, ?)"
            )
            .bind(version)
            .bind(description)
            .execute(pool)
            .await?;
        }
        return Ok(());
    }

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
    // SQLite 不支持 ALTER TABLE ADD COLUMN IF NOT EXISTS，重复执行会报 "duplicate column name"。
    // 使用 execute_tolerant 容忍已知错误，但通过 stderr 日志保留可追溯性（依据 E-08 规范）。
    if current < 2 {
        execute_tolerant(
            pool,
            "ALTER TABLE diseases ADD COLUMN urgency_level INTEGER DEFAULT 3 CHECK(urgency_level BETWEEN 1 AND 5)",
            "v2: diseases.urgency_level"
        ).await;
        execute_tolerant(
            pool,
            "ALTER TABLE disease_symptom ADD COLUMN is_pathognomonic INTEGER DEFAULT 0 CHECK(is_pathognomonic IN (0, 1))",
            "v2: disease_symptom.is_pathognomonic"
        ).await;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version, description)
             VALUES (2, 'Add urgency_level to diseases, is_pathognomonic to disease_symptom')"
        )
        .execute(pool)
        .await?;
    }

    // ── v3: disease_diagnostic 主键增加 species（重建表） ──
    // 表重建链路：CREATE v2 → INSERT SELECT → DROP old → RENAME v2 to old
    // - CREATE 已用 IF NOT EXISTS（幂等）→ 错误传播
    // - INSERT SELECT 可能因旧表未建而失败（旧库场景）→ execute_tolerant
    // - DROP 已用 IF EXISTS（幂等）→ 错误传播
    // - RENAME 失败需通过日志排查 → execute_tolerant
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
        .await?;
        execute_tolerant(
            pool,
            "INSERT INTO disease_diagnostic_v2 (disease_id, test_id, purpose, evidence_level, species, expected_result)
             SELECT disease_id, test_id, purpose, 'supportive', NULL, expected_result FROM disease_diagnostic",
            "v3: migrate disease_diagnostic data"
        ).await;
        sqlx::query("DROP TABLE IF EXISTS disease_diagnostic")
            .execute(pool)
            .await?;
        execute_tolerant(
            pool,
            "ALTER TABLE disease_diagnostic_v2 RENAME TO disease_diagnostic",
            "v3: rename disease_diagnostic_v2"
        ).await;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version, description)
             VALUES (3, 'Rebuild disease_diagnostic with species in PK')"
        )
        .execute(pool)
        .await?;
    }

    // ── v4: disease_treatment 主键增加 species（重建表） ──
    // 同 v3 的表重建链路策略
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
        .await?;
        execute_tolerant(
            pool,
            "INSERT INTO disease_treatment_v2 (disease_id, drug_id, line, species, notes)
             SELECT disease_id, drug_id, line, NULL, notes FROM disease_treatment",
            "v4: migrate disease_treatment data"
        ).await;
        sqlx::query("DROP TABLE IF EXISTS disease_treatment")
            .execute(pool)
            .await?;
        execute_tolerant(
            pool,
            "ALTER TABLE disease_treatment_v2 RENAME TO disease_treatment",
            "v4: rename disease_treatment_v2"
        ).await;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version, description)
             VALUES (4, 'Rebuild disease_treatment with species in PK')"
        )
        .execute(pool)
        .await?;
    }

    // ── v5: 闪卡系统 ──
    // CREATE TABLE/INDEX IF NOT EXISTS 已幂等，错误应传播而非吞没
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
        .await?;
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
        .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_flashcard_reviews_next ON flashcard_reviews(card_id, next_review)"
        )
        .execute(pool)
        .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version, description)
             VALUES (5, 'Add flashcard system tables')"
        )
        .execute(pool)
        .await?;
    }

    // ── v6: diseases 表新增 data_source / is_deleted ──
    // 同 v2，ALTER TABLE ADD COLUMN 无法用 IF NOT EXISTS，使用 execute_tolerant 容忍并日志
    if current < 6 {
        execute_tolerant(
            pool,
            "ALTER TABLE diseases ADD COLUMN data_source TEXT DEFAULT 'seed' CHECK(data_source IN ('seed', 'user', 'imported'))",
            "v6: diseases.data_source"
        ).await;
        execute_tolerant(
            pool,
            "ALTER TABLE diseases ADD COLUMN is_deleted INTEGER DEFAULT 0",
            "v6: diseases.is_deleted"
        ).await;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version, description)
             VALUES (6, 'Add data_source and is_deleted to diseases')"
        )
        .execute(pool)
        .await?;
    }

    // ── v7: 标签系统 + 四维度增强 + 治疗模块 ──
    if current < 7 {
        // diseases 新增字段（ALTER TABLE ADD COLUMN 用 execute_tolerant）
        execute_tolerant(pool, "ALTER TABLE diseases ADD COLUMN name_latin TEXT", "v7: diseases.name_latin").await;
        execute_tolerant(pool, "ALTER TABLE diseases ADD COLUMN pathogenic_type TEXT", "v7: diseases.pathogenic_type").await;
        execute_tolerant(pool, "ALTER TABLE diseases ADD COLUMN epidemiology TEXT", "v7: diseases.epidemiology").await;
        execute_tolerant(pool, "ALTER TABLE diseases ADD COLUMN body_system TEXT", "v7: diseases.body_system").await;
        execute_tolerant(pool, "ALTER TABLE diseases ADD COLUMN physiological_basis TEXT", "v7: diseases.physiological_basis").await;

        // symptoms 新增字段
        execute_tolerant(pool, "ALTER TABLE symptoms ADD COLUMN physiological_basis TEXT", "v7: symptoms.physiological_basis").await;

        // drugs 新增字段
        execute_tolerant(pool, "ALTER TABLE drugs ADD COLUMN mechanism_of_action TEXT", "v7: drugs.mechanism_of_action").await;
        execute_tolerant(pool, "ALTER TABLE drugs ADD COLUMN pk_pd TEXT", "v7: drugs.pk_pd").await;
        execute_tolerant(pool, "ALTER TABLE drugs ADD COLUMN adverse_mechanism TEXT", "v7: drugs.adverse_mechanism").await;

        // 标签系统（CREATE TABLE IF NOT EXISTS 已幂等 → 错误传播）
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS tags (
                id TEXT PRIMARY KEY,
                name_zh TEXT NOT NULL,
                name_en TEXT,
                tag_group TEXT NOT NULL DEFAULT 'custom',
                emergency_level TEXT CHECK(emergency_level IS NULL OR emergency_level IN ('red','orange','yellow','green')),
                clinical_action TEXT,
                textbook_logic TEXT,
                typical_scenario TEXT,
                color TEXT,
                created_at TEXT DEFAULT (datetime('now'))
            )"
        ).execute(pool).await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS entity_tags (
                entity_type TEXT NOT NULL CHECK(entity_type IN ('disease','symptom','drug','treatment','case')),
                entity_id TEXT NOT NULL,
                tag_id TEXT NOT NULL REFERENCES tags(id),
                PRIMARY KEY (entity_type, entity_id, tag_id)
            )"
        ).execute(pool).await?;

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
        ).execute(pool).await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS disease_treatment_map (
                disease_id TEXT REFERENCES diseases(id),
                treatment_id TEXT REFERENCES treatments(id),
                line TEXT CHECK(line IN ('first','second','adjunctive')),
                species TEXT,
                notes TEXT,
                PRIMARY KEY (disease_id, treatment_id, species)
            )"
        ).execute(pool).await?;

        // 索引（CREATE INDEX IF NOT EXISTS 已幂等 → 错误传播）
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_entity_tags_type ON entity_tags(entity_type, entity_id)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_entity_tags_tag ON entity_tags(tag_id)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tags_group ON tags(tag_group)").execute(pool).await?;

        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version, description)
             VALUES (7, 'Tag system + disease/symptom/drug enhancements + treatments module')"
        )
        .execute(pool)
        .await?;
    }

    // ── v8: 病例全文搜索（已废弃，由 v10 删除虚表） ──
    // 历史上 v8 创建了 cases_fts 虚表，但 search.rs 始终使用 LIKE 查询而非 FTS5 MATCH。
    // 为了消除资源浪费，v10 把虚表 DROP，v8 不再创建。
    if current < 8 {
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version, description)
             VALUES (8, 'Add cases_fts for full-text search on cases (deprecated in v10)')"
        )
        .execute(pool)
        .await?;
    }

    // ── v9: tags.name_zh 去掉 UNIQUE，允许不同维度使用相同中文标签名 ──
    if current >= 7 && current < 9 {
        sqlx::query("PRAGMA foreign_keys = OFF").execute(pool).await?;
        let mut tx = pool.begin().await?;
        sqlx::query(
            "CREATE TABLE tags_v2 (
                id TEXT PRIMARY KEY,
                name_zh TEXT NOT NULL,
                name_en TEXT,
                tag_group TEXT NOT NULL DEFAULT 'custom',
                emergency_level TEXT CHECK(emergency_level IS NULL OR emergency_level IN ('red','orange','yellow','green')),
                clinical_action TEXT,
                textbook_logic TEXT,
                typical_scenario TEXT,
                color TEXT,
                created_at TEXT DEFAULT (datetime('now'))
            )"
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO tags_v2 (
                id, name_zh, name_en, tag_group, emergency_level,
                clinical_action, textbook_logic, typical_scenario, color, created_at
            )
            SELECT
                id, name_zh, name_en, tag_group, emergency_level,
                clinical_action, textbook_logic, typical_scenario, color, created_at
            FROM tags"
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query("DROP TABLE tags").execute(&mut *tx).await?;
        sqlx::query("ALTER TABLE tags_v2 RENAME TO tags")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tags_group ON tags(tag_group)")
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version, description)
             VALUES (9, 'Rebuild tags table without UNIQUE(name_zh)')"
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        sqlx::query("PRAGMA foreign_keys = ON").execute(pool).await?;
    }

    // ── v10: 删除未使用的 FTS5 虚表 ──
    // schema.sql 中曾定义 diseases_fts / symptoms_fts / drugs_fts / cases_fts 四个 FTS5 虚表，
    // 但 search.rs 始终使用 LIKE 查询而非 FTS5 MATCH，这些虚表从未被使用。
    // FTS5 虚表占用存储空间且需要 rebuild 维护，属于资源浪费，予以删除。
    // DROP TABLE IF EXISTS 已幂等，错误应传播。
    if current < 10 {
        sqlx::query("DROP TABLE IF EXISTS diseases_fts").execute(pool).await?;
        sqlx::query("DROP TABLE IF EXISTS symptoms_fts").execute(pool).await?;
        sqlx::query("DROP TABLE IF EXISTS drugs_fts").execute(pool).await?;
        sqlx::query("DROP TABLE IF EXISTS cases_fts").execute(pool).await?;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version, description)
             VALUES (10, 'Drop unused FTS5 virtual tables (diseases_fts, symptoms_fts, drugs_fts, cases_fts)')"
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// 种子数据导入：版本不匹配时全量重建。
async fn import_seed_data(pool: &DbPool) -> anyhow::Result<()> {
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

    let seed = strip_sql_comments(include_str!("../../data/seed/001_initial.sql"));

    // 按事务执行：先清空，再导入。
    // 关键：循环中遇到失败时 *立即* 返回错误，事务 drop 时自动回滚，
    // 从而 `seed_data_version` 不会被写入 → 下次启动会重试。
    let mut tx = pool.begin().await?;

    // 清空数据表（保留表结构）。容忍 "no such table"（WAL 未持久化等边缘情况）
    for tbl in &[
        "entity_tags",
        "case_disease",
        "disease_diagnostic",
        "disease_treatment",
        "disease_treatment_map",
        "disease_symptom",
        "disease_ddx",
        "cases", "diagnostic_tests", "drugs", "symptoms", "diseases",
        "treatments", "tags",
    ] {
        let r = sqlx::query(&format!("DELETE FROM {}", tbl))
            .execute(&mut *tx)
            .await;
        match r {
            Ok(_) => {},
            Err(e) if e.to_string().contains("no such table") => {},
            Err(e) => anyhow::bail!("DELETE FROM {} failed: {}", tbl, e),
        }
    }

    // 导入种子数据：失败时用 ? 立即返回 Err
    for (idx, stmt) in split_sql_statements(&seed).into_iter().enumerate() {
        let t = stmt.trim();
        if t.is_empty() {
            continue;
        }
        sqlx::query(t).execute(&mut *tx).await.map_err(|e| {
            let snippet: String = t.chars().take(200).collect();
            anyhow::anyhow!("seed import failed at stmt #{}: {} | sql: {}", idx, e, snippet)
        })?;
    }

    // 注：历史上此处会执行 cases_fts 的 FTS rebuild，但 v10 migration 已删除 cases_fts 虚表
    // （search.rs 始终使用 LIKE 查询，FTS5 虚表属于资源浪费），此处无需再 rebuild。

    // 记录数据版本（最后一步：只有在前面全部成功的前提下才执行）
    sqlx::query(
        "INSERT OR REPLACE INTO app_meta (key, value) VALUES ('seed_data_version', ?)"
    )
    .bind(SEED_DATA_VERSION)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

/// 把最后一次 import_seed_data 的失败细节写入 app_data_dir/seed_import_error.log。
/// 由 lib.rs 的 setup hook 在 init 失败时调用，让 release 包的用户也能查错。
pub fn write_import_error_log(app: &tauri::AppHandle, err: &anyhow::Error) {
    let dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(_) => return,
    };
    let path = dir.join("seed_import_error.log");
    let body = format!(
        "vet-knowledge seed import failed at {}\n\
         SEED_DATA_VERSION: {}\n\n\
         error chain:\n{}\n\n\
         troubleshooting:\n\
         - 关闭应用，删除以下目录后重启:\n  {}\n\
         - 若问题持续，请把本文件提交到 GitHub issue\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        SEED_DATA_VERSION,
        err.chain().map(|e| format!("  - {}", e)).collect::<Vec<_>>().join("\n"),
        dir.display(),
    );
    let _ = std::fs::write(&path, body);
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
        // 只有 `;` 前的内容算一个 statement，注释和空行不会单独分割
        assert_eq!(stmts.len(), 1);
    }
}
