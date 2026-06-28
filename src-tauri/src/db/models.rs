use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
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

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Symptom {
    pub id: String,
    pub name_zh: String,
    pub name_en: Option<String>,
    pub definition: Option<String>,
    pub species_notes: Option<String>,
    pub physiological_basis: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
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

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DiagnosticTest {
    pub id: String,
    pub name_zh: String,
    pub category: Option<String>,
    pub reference_ranges: Option<String>,
    pub interpretation: Option<String>,
    pub cost_estimate: Option<f64>,
    pub turnaround_time: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
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

#[derive(Debug, Serialize, Deserialize, FromRow)]
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

#[derive(Debug, Serialize, Deserialize, FromRow)]
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
