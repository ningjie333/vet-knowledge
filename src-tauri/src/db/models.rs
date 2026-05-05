use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Disease {
    pub id: String,
    pub name_zh: String,
    pub name_en: Option<String>,
    pub category: Option<String>,
    pub species: Option<String>,
    pub overview: Option<String>,
    pub etiology: Option<String>,
    pub pathophysiology: Option<String>,
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
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Drug {
    pub id: String,
    pub name_zh: String,
    pub name_en: Option<String>,
    pub drug_class: Option<String>,
    pub indications: Option<String>,
    pub contraindications: Option<String>,
    pub side_effects: Option<String>,
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
pub struct DiseaseSymptom {
    pub disease_id: String,
    pub symptom_id: String,
    pub frequency: Option<String>,
    pub stage: Option<String>,
    pub is_pathognomonic: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DiseaseDdx {
    pub disease_id: String,
    pub ddx_id: String,
    pub distinguishing_feature: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DiseaseTreatment {
    pub disease_id: String,
    pub drug_id: String,
    pub line: Option<String>,
    pub species: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DiseaseDiagnostic {
    pub disease_id: String,
    pub test_id: String,
    pub purpose: Option<String>,
    pub evidence_level: Option<String>,
    pub species: Option<String>,
    pub expected_result: Option<String>,
}
