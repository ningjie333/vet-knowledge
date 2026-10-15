pub mod models;
pub mod sql_builder;
pub mod entity_types;

use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    Pool, Sqlite,
};
use std::path::PathBuf;
use std::str::FromStr;
use tauri::{AppHandle, Manager};

pub type DbPool = Pool<Sqlite>;

/// 连接选项：所有 PRAGMA 都在这里声明，而不是连上之后 `execute(pool)`。
///
/// 为什么必须这样：SQLite 的 PRAGMA 是**按连接**生效的，而池里有 2 条连接。启动后用
/// `execute(&pool)` 设 PRAGMA，只会命中当时被借出的那一条，另一条仍是默认值 —— 于是同一份
/// 配置在两条连接上并不一致（`cache_size`、`temp_store` 这类性能参数会静默只对一半生效）。
///
/// 外键这一项 sqlx 的连接选项本来就默认开启（实测退回旧写法也不会丢），所以真正的事故点是
/// v9 迁移：`PRAGMA foreign_keys = OFF` 打在一条连接上，`pool.begin()` 的事务却跑在另一条
/// 仍开启外键的连接上，`DROP TABLE tags` 直接 "FOREIGN KEY constraint failed"，
/// 凡是数据库停在 v9 之前的用户升级后都启动不了（现场证据：app_data_dir/seed_import_error.log）。
///
/// 测试也走这个函数，保证被测库的外键行为与生产一致。
pub fn sqlite_options(db_url: &str) -> anyhow::Result<SqliteConnectOptions> {
    Ok(SqliteConnectOptions::from_str(db_url)?
        .create_if_missing(true)
        // 用 DELETE 而非 WAL：WAL 需要 .db-shm 共享内存文件，沙箱化环境会拦截它导致
        // disk I/O error；DELETE 无 SHM 依赖，单用户桌面应用足够。
        .journal_mode(SqliteJournalMode::Delete)
        // DELETE 模式下 NORMAL 在断电时有损坏全库风险，用 FULL 保崩溃安全。
        .synchronous(SqliteSynchronous::Full)
        .foreign_keys(true)
        .pragma("cache_size", "-64000")
        .pragma("temp_store", "MEMORY"))
}

/// 建立连接池。生产与测试共用这一条构造路径 —— 否则测试只钉住了 PRAGMA 本身，
/// 却钉不住"init 到底有没有用它"。
pub async fn create_pool(db_url: &str) -> anyhow::Result<DbPool> {
    Ok(SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(sqlite_options(db_url)?)
        .await?)
}

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
/// v20: entity_tags 覆盖全部可打标签实体（disease/symptom/drug/treatment），
///      此前只有 disease 有标签行 → 症状/药物/治疗的标签区永远空白。
const SEED_DATA_VERSION: i64 = 22;

pub async fn init(app: &AppHandle) -> anyhow::Result<DbPool> {
    // debug 构建用项目根目录下的 data/（src-tauri 的父目录），
    // 避免 TRAE Sandbox 拦截 app_data_dir 访问，同时避开 Tauri file watcher 监控范围。
    // release 构建用标准 app_data_dir（生产环境无沙箱限制）。
    let app_dir = if cfg!(debug_assertions) {
        // CARGO_MANIFEST_DIR = src-tauri/ 的绝对路径，parent() = 项目根
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap_or(&manifest_dir);
        project_root.join("data")
    } else {
        app.path().app_data_dir().unwrap_or_else(|_| {
            let mut p = std::env::current_dir().unwrap_or_default();
            p.push("data");
            p
        })
    };
    std::fs::create_dir_all(&app_dir)?;

    let db_path = app_dir.join("vet_knowledge.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display()).replace('\\', "/");

    let pool = create_pool(&db_url).await?;

    // SQLite PRAGMA（外键 / journal_mode / synchronous / cache_size / temp_store）全部在
    // sqlite_options() 里按连接声明 —— 见该函数注释：用 execute(&pool) 设 PRAGMA 只会命中
    // 池里被借出的那一条连接，另一条仍是默认值，约束强度不确定。

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
            (11, "Drop unused learning_progress table"),
            (12, "Add drug_interactions table"),
            (13, "Add PubChem columns to drugs (were only ever in schema.sql)"),
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
        // 关外键 → 重建 → 开外键，三步必须发生在**同一条连接**上：PRAGMA 是按连接的，
        // 原先用 `execute(pool)` 会把 PRAGMA 打到另一条连接，事务所在连接仍开着外键，
        // `DROP TABLE tags` 便因 entity_tags 引用它而报 FOREIGN KEY constraint failed，
        // 结果是所有停留在 v9 之前的存量库升级后启动即崩。
        use sqlx::Connection as _;
        let mut conn = pool.acquire().await?;
        sqlx::query("PRAGMA foreign_keys = OFF").execute(&mut *conn).await?;
        let mut tx = conn.begin().await?;
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
        sqlx::query("PRAGMA foreign_keys = ON").execute(&mut *conn).await?;
    }

    // ── v12: 药物相互作用表 ──
    if current < 12 {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS drug_interactions (
                drug_a_id TEXT NOT NULL REFERENCES drugs(id),
                drug_b_id TEXT NOT NULL REFERENCES drugs(id),
                severity TEXT NOT NULL CHECK(severity IN ('major','moderate','mild')),
                description TEXT,
                PRIMARY KEY (drug_a_id, drug_b_id)
            )"
        ).execute(pool).await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_drug_interactions_a ON drug_interactions(drug_a_id)"
        ).execute(pool).await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_drug_interactions_b ON drug_interactions(drug_b_id)"
        ).execute(pool).await?;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version, description)
             VALUES (12, 'Add drug_interactions table')"
        )
        .execute(pool)
        .await?;
    }

    // ── v13: drugs 补齐 PubChem 化学性质列 ──
    // 这 6 列只写在 schema.sql 里，历史上没有任何迁移补过 → 全新库有、存量库没有。
    // 而种子 SQL 是按当前列名 INSERT 的，于是老库升级后导入直接失败
    // （"table drugs has no column named pubchem_cid"），应用弹错误框后退出。
    if current < 13 {
        use sqlx::Row;
        // pragma_table_info() 对不存在的表会直接报错，先确认表在
        let has_drugs: i64 = sqlx::query("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='drugs'")
            .fetch_one(pool)
            .await?
            .get::<i64, _>(0);
        if has_drugs > 0 {
            for (col, ty) in [
                ("pubchem_cid", "INTEGER"),
                ("molecular_formula", "TEXT"),
                ("molecular_weight", "REAL"),
                ("smiles", "TEXT"),
                ("inchikey", "TEXT"),
                ("iupac_name", "TEXT"),
            ] {
                let have: i64 = sqlx::query(
                    "SELECT COUNT(*) FROM pragma_table_info('drugs') WHERE name = ?",
                )
                .bind(col)
                .fetch_one(pool)
                .await?
                .get::<i64, _>(0);
                if have == 0 {
                    sqlx::query(&format!("ALTER TABLE drugs ADD COLUMN {} {}", col, ty))
                        .execute(pool)
                        .await?;
                }
            }
        }
        // 版本号无条件落表：否则表缺失的库每次启动都重跑一遍
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version, description)
             VALUES (13, 'Add PubChem columns to drugs (were only ever in schema.sql)')"
        )
        .execute(pool)
        .await?;
    }

    // ── v11: 删除未使用的 learning_progress 表 ──
    // schema.sql 中曾定义 learning_progress 表，但没有任何命令读写它，属于死表。
    // DROP TABLE IF EXISTS 已幂等，错误应传播。
    if current < 11 {
        sqlx::query("DROP TABLE IF EXISTS learning_progress").execute(pool).await?;
        sqlx::query(
            "INSERT OR IGNORE INTO schema_migrations (version, description)
             VALUES (11, 'Drop unused learning_progress table')"
        )
        .execute(pool)
        .await?;
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
        "drug_interactions",
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

    /// 在临时目录里开一个文件库的 URL（内存库每条连接是各自独立的库，测不了跨连接行为）。
    fn temp_db_url(tag: &str) -> (PathBuf, String) {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static SEQ: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "vet-knowledge-dbtest-{}-{}-{}",
            tag,
            std::process::id(),
            SEQ.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).expect("创建临时目录");
        let url = format!(
            "sqlite://{}?mode=rwc",
            dir.join("vet_knowledge.db").display()
        )
        .replace('\\', "/");
        (dir, url)
    }

    /// 池里**每一条**连接都必须开启外键。
    ///
    /// 旧写法是连上库之后 `execute(&pool)` 发 PRAGMA —— 那只会命中当时被借出的那一条连接，
    /// 另一条仍是 SQLite 默认的"外键关闭"。于是同一个库里哪些写入受约束，取决于这次借到哪条
    /// 连接；v9 迁移的 `PRAGMA foreign_keys = OFF` 也正是这样打在空处，导致老库升级即崩。
    #[tokio::test]
    async fn test_every_pool_connection_gets_the_same_pragmas() {
        use sqlx::Row;
        let (dir, url) = temp_db_url("fk-per-conn");
        let pool = create_pool(&url).await.expect("连接失败");

        // 同时持有两条连接，确保检查的是两个不同连接而不是同一个借了两次
        let mut held = Vec::new();
        for _ in 0..2 {
            held.push(pool.acquire().await.expect("获取连接"));
        }
        assert_eq!(held.len(), 2, "池子没有真正开出 2 条连接，测试会失去意义");

        let mut per_conn = Vec::new();
        for conn in held.iter_mut() {
            let fk_row = sqlx::query("PRAGMA foreign_keys")
                .fetch_one(&mut **conn).await.expect("读取 foreign_keys 失败");
            let fk = fk_row.try_get::<i64, _>(0).unwrap_or(-999);
            let cache_row = sqlx::query("PRAGMA cache_size")
                .fetch_one(&mut **conn).await.expect("读取 cache_size 失败");
            let cache = cache_row.try_get::<i64, _>(0).unwrap_or(-999);
            per_conn.push((fk, cache));
        }
        // 外键：sqlx 默认开，这里只是守住"别被谁关掉"
        // cache_size：非 sqlx 默认值，只有走连接选项才会两条连接都是 -64000
        for (fk, cache) in &per_conn {
            assert_eq!(*fk, 1, "该连接没开外键: {:?}", per_conn);
            assert_eq!(
                *cache, -64000,
                "该连接的 cache_size 不是配置值，说明按连接的 PRAGMA 又只落到了一条连接上: {:?}",
                per_conn
            );
        }

        drop(held);
        pool.close().await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 存量库升级回归：停在 v8 的库（tags 带 UNIQUE(name_zh)、entity_tags 有引用行）
    /// 必须能一路迁移到最新版本且不丢标签数据。
    ///
    /// 回归的正是线上真实故障：v9 重建 tags 时 `DROP TABLE tags` 报
    /// FOREIGN KEY constraint failed，凡数据库早于 v9 的安装包升级后启动即崩
    /// （现场证据是 app_data_dir 里的 seed_import_error.log）。
    #[tokio::test]
    async fn test_upgrade_from_pre_v9_database_keeps_tag_rows() {
        let (dir, url) = temp_db_url("pre-v9");

        {
            let setup = SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(sqlite_options(&url).expect("构造连接选项失败"))
                .await
                .expect("建旧库失败");
            sqlx::query(
                "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT, description TEXT)",
            )
            .execute(&setup)
            .await
            .unwrap();
            for v in 1..=8 {
                sqlx::query("INSERT INTO schema_migrations (version) VALUES (?)")
                    .bind(v)
                    .execute(&setup)
                    .await
                    .unwrap();
            }
            // v8 形态：列齐全，但 name_zh 带 UNIQUE —— 这正是 v9 要拆掉的约束
            sqlx::query(
                "CREATE TABLE tags (
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
                )",
            )
            .execute(&setup)
            .await
            .unwrap();
            sqlx::query(
                "CREATE TABLE entity_tags (entity_type TEXT NOT NULL, entity_id TEXT NOT NULL, \
                 tag_id TEXT NOT NULL REFERENCES tags(id), PRIMARY KEY (entity_type, entity_id, tag_id))",
            )
            .execute(&setup)
            .await
            .unwrap();
            sqlx::query("INSERT INTO tags (id, name_zh, tag_group) VALUES ('body_resp', '呼吸系统', 'body_system')")
                .execute(&setup)
                .await
                .unwrap();
            sqlx::query("INSERT INTO entity_tags (entity_type, entity_id, tag_id) VALUES ('disease', 'dis_001', 'body_resp')")
                .execute(&setup)
                .await
                .unwrap();
            setup.close().await;
        }

        let pool = create_pool(&url).await.expect("连接旧库失败");

        apply_migrations(&pool).await.expect("从 v8 迁移到最新版本失败（回归）");

        use sqlx::Row;
        let version: i64 = sqlx::query("SELECT MAX(version) FROM schema_migrations")
            .fetch_one(&pool)
            .await
            .unwrap()
            .get(0);
        assert_eq!(version, 13, "迁移没有走到最新版本");

        let tags: i64 = sqlx::query("SELECT COUNT(*) FROM tags")
            .fetch_one(&pool)
            .await
            .unwrap()
            .get(0);
        let links: i64 = sqlx::query("SELECT COUNT(*) FROM entity_tags")
            .fetch_one(&pool)
            .await
            .unwrap()
            .get(0);
        assert_eq!((tags, links), (1, 1), "重建 tags 表时丢了用户已有的标签数据");

        // 迁移结束后外键必须恢复生效（PRAGMA OFF 不能泄漏到后续使用）
        let bad = sqlx::query("INSERT INTO entity_tags (entity_type, entity_id, tag_id) VALUES ('disease', 'dis_x', 'nope')")
            .execute(&pool)
            .await;
        assert!(bad.is_err(), "迁移后外键没有恢复，坏数据可以写进库");

        pool.close().await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 存量库升级后必须能真正吃下当前种子 SQL。
    ///
    /// 回归的是线上第二个"升级即崩"：drugs 的 6 个 PubChem 列只写在 schema.sql 里、
    /// 从来没有任何迁移补过，于是老库结构缺列，而种子按当前列名 INSERT →
    /// "table drugs has no column named pubchem_cid"，应用弹错误框后退出。
    /// 光比对 schema.sql 与种子是查不出来的（两边都有这些列），必须真的从老结构走一遍。
    #[tokio::test]
    async fn test_legacy_drugs_schema_upgrades_then_accepts_seed() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("内存库连接失败");
        sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();

        for stmt in split_sql_statements(&strip_sql_comments(include_str!(
            "../../data/seed/schema.sql"
        ))) {
            let t = stmt.trim();
            if !t.is_empty() {
                sqlx::query(t).execute(&pool).await.expect("schema.sql 执行失败");
            }
        }

        // 退回 v13 之前的 drugs 结构：没有那 6 个 PubChem 列
        sqlx::query(
            "CREATE TABLE drugs_legacy (
                id TEXT PRIMARY KEY, name_zh TEXT NOT NULL, name_en TEXT, drug_class TEXT,
                mechanism_of_action TEXT, pk_pd TEXT, indications TEXT, contraindications TEXT,
                side_effects TEXT, adverse_mechanism TEXT, species_dosing TEXT
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("DROP TABLE drugs").execute(&pool).await.unwrap();
        sqlx::query("ALTER TABLE drugs_legacy RENAME TO drugs").execute(&pool).await.unwrap();

        // 声明这是一个"迁移到 v8 为止"的存量库
        for v in 1..=8 {
            sqlx::query("INSERT OR IGNORE INTO schema_migrations (version) VALUES (?)")
                .bind(v)
                .execute(&pool)
                .await
                .unwrap();
        }

        apply_migrations(&pool).await.expect("存量库迁移失败（回归）");


        // 最后一道：拿真实种子 SQL 里的第一条 drugs INSERT 打进去，必须成功
        let seed = strip_sql_comments(include_str!("../../data/seed/001_initial.sql"));
        let drugs_insert = split_sql_statements(&seed)
            .into_iter()
            .map(|s| s.trim().to_string())
            .find(|s| s.starts_with("INSERT INTO drugs ("))
            .expect("种子里找不到 drugs 的 INSERT");
        sqlx::query(&drugs_insert)
            .execute(&pool)
            .await
            .expect("迁移后的库仍然拒绝真实种子 INSERT（列不匹配）");

        pool.close().await;
    }

    /// 种子 SQL 端到端冒烟测试：schema.sql + 001_initial.sql 全量执行，
    /// 外键打开（应用启动时就是这个配置），确认：
    /// - 没有任何外键违规（漏注册标签会在这里爆炸）
    /// - 四类可打标签的实体都有 entity_tags 行（"标签区永远空白"的根因）
    #[tokio::test]
    async fn test_seed_sql_imports_with_foreign_keys_on() {
        use sqlx::Row;

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("内存 SQLite 连接失败");
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .expect("开启外键失败");

        for script in [
            include_str!("../../data/seed/schema.sql"),
            include_str!("../../data/seed/001_initial.sql"),
        ] {
            let cleaned = strip_sql_comments(script);
            for stmt in split_sql_statements(&cleaned) {
                let t = stmt.trim();
                if t.is_empty() {
                    continue;
                }
                if let Err(e) = sqlx::query(t).execute(&pool).await {
                    let snippet: String = t.chars().take(160).collect();
                    panic!("种子 SQL 执行失败: {} | sql: {}", e, snippet);
                }
            }
        }

        let violations = sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&pool)
            .await
            .expect("foreign_key_check 查询失败");
        assert!(violations.is_empty(), "种子数据存在外键违规: {} 条", violations.len());

        let rows = sqlx::query("SELECT entity_type, COUNT(*) FROM entity_tags GROUP BY entity_type")
            .fetch_all(&pool)
            .await
            .expect("entity_tags 统计失败");
        let counts: std::collections::HashMap<String, i64> = rows
            .iter()
            .map(|r| (r.get::<String, _>(0), r.get::<i64, _>(1)))
            .collect();

        // 只有疾病有标签行 = 症状/药物/治疗的标签区永远空白（本次修复的根因）
        for entity_type in ["disease", "symptom", "drug", "treatment"] {
            let n = counts.get(entity_type).copied().unwrap_or(0);
            assert!(n > 0, "种子数据里 {} 没有任何 entity_tags 行", entity_type);
        }
        // entity_tags 的 entity_type 必须都在注册表里（schema.sql 的 CHECK 约束已经保证，
        // 这里再对一次账，防止两处取值漂移）
        for entity_type in counts.keys() {
            assert!(
                entity_types::is_valid_entity_type(entity_type),
                "entity_tags 使用了未注册的 entity_type: {}",
                entity_type
            );
        }
    }
}
