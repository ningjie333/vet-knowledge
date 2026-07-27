use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::sqlite::SqliteRow;
use sqlx::Row;

// ── Row 构造函数 ──
// 从 SqliteRow 手动构造实体，用于 JOIN 查询（query() 而非 query_as）。
// 统一放在 models.rs，避免各 commands 模块重复定义。

#[inline]
pub fn row_from_disease(r: &SqliteRow) -> Disease {
    Disease {
        id: r.get("id"),
        name_zh: r.get("name_zh"),
        name_en: r.get("name_en"),
        name_latin: r.get("name_latin"),
        category: r.get("category"),
        species: r.get("species"),
        body_system: r.get("body_system"),
        pathogenic_type: r.get("pathogenic_type"),
        epidemiology: r.get("epidemiology"),
        overview: r.get("overview"),
        etiology: r.get("etiology"),
        pathophysiology: r.get("pathophysiology"),
        physiological_basis: r.get("physiological_basis"),
        prognosis: r.get("prognosis"),
        difficulty: r.get("difficulty"),
        urgency_level: r.get("urgency_level"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}

#[inline]
pub fn row_from_symptom(r: &SqliteRow) -> Symptom {
    Symptom {
        id: r.get("id"),
        name_zh: r.get("name_zh"),
        name_en: r.get("name_en"),
        definition: r.get("definition"),
        species_notes: r.get("species_notes"),
        physiological_basis: r.get("physiological_basis"),
    }
}

#[inline]
pub fn row_from_drug(r: &SqliteRow) -> Drug {
    Drug {
        id: r.get("id"),
        name_zh: r.get("name_zh"),
        name_en: r.get("name_en"),
        drug_class: r.get("drug_class"),
        mechanism_of_action: r.get("mechanism_of_action"),
        pk_pd: r.get("pk_pd"),
        indications: r.get("indications"),
        contraindications: r.get("contraindications"),
        side_effects: r.get("side_effects"),
        adverse_mechanism: r.get("adverse_mechanism"),
        species_dosing: r.get("species_dosing"),
    }
}

#[inline]
pub fn row_from_test(r: &SqliteRow) -> DiagnosticTest {
    DiagnosticTest {
        id: r.get("id"),
        name_zh: r.get("name_zh"),
        category: r.get("category"),
        reference_ranges: r.get("reference_ranges"),
        interpretation: r.get("interpretation"),
        cost_estimate: r.get("cost_estimate"),
        turnaround_time: r.get("turnaround_time"),
    }
}

#[inline]
pub fn row_from_treatment(r: &SqliteRow) -> Treatment {
    Treatment {
        id: r.get("id"),
        name_zh: r.get("name_zh"),
        name_en: r.get("name_en"),
        therapy_type: r.get("therapy_type"),
        principle: r.get("principle"),
        procedure_text: r.get("procedure_text"),
        physiological_basis: r.get("physiological_basis"),
        prognosis_assessment: r.get("prognosis_assessment"),
        created_at: r.get("created_at"),
    }
}

#[inline]
pub fn row_from_tag(r: &SqliteRow) -> Tag {
    Tag {
        id: r.get("id"),
        name_zh: r.get("name_zh"),
        name_en: r.get("name_en"),
        tag_group: r.get("tag_group"),
        emergency_level: r.get("emergency_level"),
        clinical_action: r.get("clinical_action"),
        textbook_logic: r.get("textbook_logic"),
        typical_scenario: r.get("typical_scenario"),
        color: r.get("color"),
        created_at: r.get("created_at"),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Disease {
    pub id: String,
    pub name_zh: String,
    pub name_en: Option<String>,
    pub name_latin: Option<String>,
    pub category: Option<String>,
    pub species: Option<String>,
    pub body_system: Option<String>,
    pub pathogenic_type: Option<String>,
    pub epidemiology: Option<String>,
    pub overview: Option<String>,
    pub etiology: Option<String>,
    pub pathophysiology: Option<String>,
    pub physiological_basis: Option<String>,
    pub prognosis: Option<String>,
    pub difficulty: Option<String>,
    pub urgency_level: Option<i64>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Symptom {
    pub id: String,
    pub name_zh: String,
    pub name_en: Option<String>,
    pub definition: Option<String>,
    pub species_notes: Option<String>,
    pub physiological_basis: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Drug {
    pub id: String,
    pub name_zh: String,
    pub name_en: Option<String>,
    pub drug_class: Option<String>,
    pub mechanism_of_action: Option<String>,
    pub pk_pd: Option<String>,
    pub indications: Option<String>,
    pub contraindications: Option<String>,
    pub side_effects: Option<String>,
    pub adverse_mechanism: Option<String>,
    pub species_dosing: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DiagnosticTest {
    pub id: String,
    pub name_zh: String,
    pub category: Option<String>,
    pub reference_ranges: Option<String>,
    pub interpretation: Option<String>,
    pub cost_estimate: Option<f64>,
    pub turnaround_time: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Case {
    pub id: String,
    pub title: String,
    pub species: Option<String>,
    pub breed: Option<String>,
    pub age: Option<f64>,
    pub weight: Option<f64>,
    pub chief_complaint: Option<String>,
    pub history: Option<String>,
    pub physical_exam: Option<String>,
    pub lab_results: Option<String>,
    pub imaging: Option<String>,
    pub diagnosis: Option<String>,
    pub treatment: Option<String>,
    pub outcome: Option<String>,
    pub learning_points: Option<String>,
    pub difficulty: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Treatment {
    pub id: String,
    pub name_zh: String,
    pub name_en: Option<String>,
    pub therapy_type: Option<String>,
    pub principle: Option<String>,
    pub procedure_text: Option<String>,
    pub physiological_basis: Option<String>,
    pub prognosis_assessment: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Tag {
    pub id: String,
    pub name_zh: String,
    pub name_en: Option<String>,
    pub tag_group: String,
    pub emergency_level: Option<String>,
    pub clinical_action: Option<String>,
    pub textbook_logic: Option<String>,
    pub typical_scenario: Option<String>,
    pub color: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiseaseWithSymptom {
    pub disease: Disease,
    pub frequency: String,
    pub stage: String,
    pub is_pathognomonic: i64,
}

// ── DTO（数据传输对象，不映射数据库表） ──
// 诊断推理引擎的输出结构，由 engine.rs 构造，通过 diagnose 命令返回前端

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuggestion {
    pub test_id: String,
    pub test_name: String,
    pub purpose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosisCandidate {
    pub disease_id: String,
    pub disease_name: String,
    pub match_score: f64,
    pub input_coverage: f64,
    pub matched_symptoms: Vec<String>,
    pub missing_key_symptoms: Vec<String>,
    pub suggested_tests: Vec<TestSuggestion>,
    pub distinguishing_features: Vec<String>,
}

// 搜索命令的返回结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub entity_type: String,
    pub entity_id: String,
    pub title: String,
    pub snippet: String,
    pub relevance: f64,
}
