-- =============================================
-- 兽医知识库 — 数据库 Schema v3
-- =============================================

CREATE TABLE IF NOT EXISTS diseases (
    id TEXT PRIMARY KEY,
    name_zh TEXT NOT NULL,
    name_en TEXT,
    name_latin TEXT,
    category TEXT,
    species TEXT,
    body_system TEXT,
    pathogenic_type TEXT,
    epidemiology TEXT,
    overview TEXT,
    etiology TEXT,
    pathophysiology TEXT,
    physiological_basis TEXT,
    prognosis TEXT,
    difficulty TEXT DEFAULT 'intermediate',
    urgency_level INTEGER DEFAULT 3 CHECK(urgency_level BETWEEN 1 AND 5),
    data_source TEXT DEFAULT 'seed' CHECK(data_source IN ('seed', 'user', 'imported')),
    is_deleted INTEGER DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS symptoms (
    id TEXT PRIMARY KEY,
    name_zh TEXT NOT NULL,
    name_en TEXT,
    definition TEXT,
    species_notes TEXT,
    physiological_basis TEXT
);

CREATE TABLE IF NOT EXISTS drugs (
    id TEXT PRIMARY KEY,
    name_zh TEXT NOT NULL,
    name_en TEXT,
    drug_class TEXT,
    mechanism_of_action TEXT,
    pk_pd TEXT,
    indications TEXT,
    contraindications TEXT,
    side_effects TEXT,
    adverse_mechanism TEXT,
    species_dosing TEXT
);

CREATE TABLE IF NOT EXISTS diagnostic_tests (
    id TEXT PRIMARY KEY,
    name_zh TEXT NOT NULL,
    category TEXT,
    reference_ranges TEXT,
    interpretation TEXT,
    cost_estimate REAL,
    turnaround_time INTEGER
);

CREATE TABLE IF NOT EXISTS cases (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    species TEXT,
    breed TEXT,
    age REAL,
    weight REAL,
    chief_complaint TEXT,
    history TEXT,
    physical_exam TEXT,
    lab_results TEXT,
    imaging TEXT,
    diagnosis TEXT,
    treatment TEXT,
    outcome TEXT,
    learning_points TEXT,
    difficulty TEXT DEFAULT 'intermediate'
);

CREATE TABLE IF NOT EXISTS treatments (
    id TEXT PRIMARY KEY,
    name_zh TEXT NOT NULL,
    name_en TEXT,
    therapy_type TEXT CHECK(therapy_type IN ('药物疗法','手术疗法','急救重症','康复理疗','营养辅助','其他')),
    principle TEXT,
    procedure_text TEXT,
    physiological_basis TEXT,
    prognosis_assessment TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);

-- ── 标签系统 ──

CREATE TABLE IF NOT EXISTS tags (
    id TEXT PRIMARY KEY,
    name_zh TEXT NOT NULL,
    name_en TEXT,
    tag_group TEXT NOT NULL DEFAULT 'custom',
    -- tag_group: body_system / mechanism / emergency / damnit_v / species / custom
    -- emergency 子等级（仅 emergency 组有效）:
    --   red / orange / yellow / green
    emergency_level TEXT CHECK(emergency_level IS NULL OR emergency_level IN ('red','orange','yellow','green')),
    clinical_action TEXT,   -- 临床决策建议（emergency 组）
    textbook_logic TEXT,    -- 教科书逻辑说明
    typical_scenario TEXT,  -- 典型场景示例
    color TEXT,             -- 前端显示颜色
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS entity_tags (
    entity_type TEXT NOT NULL CHECK(entity_type IN ('disease','symptom','drug','treatment','case')),
    entity_id TEXT NOT NULL,
    tag_id TEXT NOT NULL REFERENCES tags(id),
    PRIMARY KEY (entity_type, entity_id, tag_id)
);

-- ── 关系表 ──

CREATE TABLE IF NOT EXISTS disease_symptom (
    disease_id TEXT REFERENCES diseases(id),
    symptom_id TEXT REFERENCES symptoms(id),
    frequency TEXT CHECK(frequency IN ('common','uncommon','rare')),
    stage TEXT,
    is_pathognomonic INTEGER DEFAULT 0 CHECK(is_pathognomonic IN (0, 1)),
    PRIMARY KEY (disease_id, symptom_id)
);

CREATE TABLE IF NOT EXISTS disease_ddx (
    disease_id TEXT REFERENCES diseases(id),
    ddx_id TEXT REFERENCES diseases(id),
    distinguishing_feature TEXT,
    PRIMARY KEY (disease_id, ddx_id)
);

CREATE TABLE IF NOT EXISTS disease_treatment (
    disease_id TEXT REFERENCES diseases(id),
    drug_id TEXT REFERENCES drugs(id),
    line TEXT CHECK(line IN ('first','second','adjunctive')),
    species TEXT,
    notes TEXT,
    PRIMARY KEY (disease_id, drug_id, species)
);

CREATE TABLE IF NOT EXISTS disease_diagnostic (
    disease_id TEXT REFERENCES diseases(id),
    test_id TEXT REFERENCES diagnostic_tests(id),
    purpose TEXT CHECK(purpose IN ('screening','confirming','monitoring')),
    evidence_level TEXT DEFAULT 'supportive' CHECK(evidence_level IN ('gold_standard','supportive','exclusionary')),
    species TEXT,
    expected_result TEXT,
    PRIMARY KEY (disease_id, test_id, species)
);

CREATE TABLE IF NOT EXISTS disease_treatment_map (
    disease_id TEXT REFERENCES diseases(id),
    treatment_id TEXT REFERENCES treatments(id),
    line TEXT CHECK(line IN ('first','second','adjunctive')),
    species TEXT,
    notes TEXT,
    PRIMARY KEY (disease_id, treatment_id, species)
);

CREATE TABLE IF NOT EXISTS case_disease (
    case_id TEXT REFERENCES cases(id),
    disease_id TEXT REFERENCES diseases(id),
    PRIMARY KEY (case_id, disease_id)
);

-- ── Schema 版本管理 ──
-- 注：曾在此处定义 diseases_fts / symptoms_fts / drugs_fts / cases_fts 四个 FTS5 虚表，
-- 但 search.rs 始终使用 LIKE 查询而非 FTS5 MATCH，这些虚表从未被使用。
-- v10 migration 已 DROP 这些虚表，schema.sql 也不再创建，避免资源浪费。

CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT DEFAULT (datetime('now')),
    description TEXT
);

-- ── 学习进度 ──

CREATE TABLE IF NOT EXISTS learning_progress (
    entity_type TEXT,
    entity_id TEXT,
    mastery_level REAL DEFAULT 0,
    review_count INTEGER DEFAULT 0,
    last_reviewed TEXT,
    next_review TEXT,
    PRIMARY KEY (entity_type, entity_id)
);

-- ── 闪卡系统 ──

CREATE TABLE IF NOT EXISTS flashcards (
    id TEXT PRIMARY KEY,
    front TEXT NOT NULL,
    back TEXT NOT NULL,
    card_type TEXT NOT NULL CHECK(card_type IN ('disease', 'symptom', 'drug', 'custom')),
    entity_id TEXT,
    difficulty REAL DEFAULT 0.5 CHECK(difficulty BETWEEN 0 AND 1),
    created_at TEXT DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS flashcard_reviews (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    card_id TEXT NOT NULL REFERENCES flashcards(id) ON DELETE CASCADE,
    quality INTEGER NOT NULL CHECK(quality BETWEEN 0 AND 5),
    reviewed_at TEXT DEFAULT (datetime('now')),
    interval_days REAL DEFAULT 0,
    ease_factor REAL DEFAULT 2.5,
    next_review TEXT NOT NULL
);

-- ── 索引 ──

CREATE INDEX IF NOT EXISTS idx_disease_symptom_d ON disease_symptom(disease_id);
CREATE INDEX IF NOT EXISTS idx_disease_symptom_s ON disease_symptom(symptom_id);
CREATE INDEX IF NOT EXISTS idx_disease_ddx ON disease_ddx(disease_id);
CREATE INDEX IF NOT EXISTS idx_disease_treatment ON disease_treatment(disease_id);
CREATE INDEX IF NOT EXISTS idx_disease_diagnostic ON disease_diagnostic(disease_id);
CREATE INDEX IF NOT EXISTS idx_case_species ON cases(species);
CREATE INDEX IF NOT EXISTS idx_flashcard_reviews_next ON flashcard_reviews(card_id, next_review);
CREATE INDEX IF NOT EXISTS idx_entity_tags_type ON entity_tags(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_entity_tags_tag ON entity_tags(tag_id);
CREATE INDEX IF NOT EXISTS idx_tags_group ON tags(tag_group);
