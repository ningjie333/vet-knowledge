-- =============================================
-- 兽医知识库 — 数据库 Schema
-- =============================================

CREATE TABLE IF NOT EXISTS diseases (
    id TEXT PRIMARY KEY,
    name_zh TEXT NOT NULL,
    name_en TEXT,
    category TEXT,
    species TEXT,
    overview TEXT,
    etiology TEXT,
    pathophysiology TEXT,
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
    species_notes TEXT
);

CREATE TABLE IF NOT EXISTS drugs (
    id TEXT PRIMARY KEY,
    name_zh TEXT NOT NULL,
    name_en TEXT,
    drug_class TEXT,
    indications TEXT,
    contraindications TEXT,
    side_effects TEXT,
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

-- 关系表
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

CREATE TABLE IF NOT EXISTS case_disease (
    case_id TEXT REFERENCES cases(id),
    disease_id TEXT REFERENCES diseases(id),
    PRIMARY KEY (case_id, disease_id)
);

-- 全文搜索
CREATE VIRTUAL TABLE IF NOT EXISTS diseases_fts USING fts5(
    name_zh, name_en, overview, content='diseases', content_rowid='rowid'
);
CREATE VIRTUAL TABLE IF NOT EXISTS symptoms_fts USING fts5(
    name_zh, name_en, definition, content='symptoms', content_rowid='rowid'
);
CREATE VIRTUAL TABLE IF NOT EXISTS drugs_fts USING fts5(
    name_zh, name_en, drug_class, content='drugs', content_rowid='rowid'
);

-- Schema 版本管理（支持未来迁移）
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT DEFAULT (datetime('now')),
    description TEXT
);
INSERT OR IGNORE INTO schema_migrations (version, description) VALUES (2, 'Add urgency_level, is_pathognomonic, evidence_level, species to relation tables');

-- 学习进度
CREATE TABLE IF NOT EXISTS learning_progress (
    entity_type TEXT,
    entity_id TEXT,
    mastery_level REAL DEFAULT 0,
    review_count INTEGER DEFAULT 0,
    last_reviewed TEXT,
    next_review TEXT,
    PRIMARY KEY (entity_type, entity_id)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_disease_symptom_d ON disease_symptom(disease_id);
CREATE INDEX IF NOT EXISTS idx_disease_symptom_s ON disease_symptom(symptom_id);
CREATE INDEX IF NOT EXISTS idx_disease_ddx ON disease_ddx(disease_id);
CREATE INDEX IF NOT EXISTS idx_disease_treatment ON disease_treatment(disease_id);
CREATE INDEX IF NOT EXISTS idx_disease_diagnostic ON disease_diagnostic(disease_id);
CREATE INDEX IF NOT EXISTS idx_case_species ON cases(species);
