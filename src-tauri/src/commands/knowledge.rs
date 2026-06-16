use crate::db::{DbPool, models::*};
use sqlx::Row;

// ===== 疾病 =====

#[tauri::command]
pub async fn get_diseases(
    pool: tauri::State<'_, DbPool>,
    species: Option<String>,
    category: Option<String>,
) -> Result<Vec<Disease>, String> {
    let species = species.unwrap_or_default();
    let category = category.unwrap_or_default();
    let has_species = !species.is_empty();
    let has_category = !category.is_empty();

    let mut query = String::from("SELECT * FROM diseases WHERE 1=1");
    if has_species { query.push_str(" AND species LIKE ?"); }
    if has_category { query.push_str(" AND category LIKE ?"); }
    query.push_str(" ORDER BY name_zh");

    let mut q = sqlx::query_as::<_, Disease>(&query);
    if has_species { q = q.bind(format!("%{}%", species)); }
    if has_category { q = q.bind(format!("%{}%", category)); }

    q.fetch_all(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_disease_by_id(
    pool: tauri::State<'_, DbPool>,
    id: String,
) -> Result<Option<Disease>, String> {
    sqlx::query_as::<_, Disease>("SELECT * FROM diseases WHERE id = ?")
        .bind(&id)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| e.to_string())
}

fn row_to_disease(r: &sqlx::sqlite::SqliteRow) -> Disease {
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

#[tauri::command]
pub async fn get_disease_symptoms(
    pool: tauri::State<'_, DbPool>,
    disease_id: String,
) -> Result<Vec<(Symptom, String, String, i64)>, String> {
    sqlx::query(
        "SELECT s.*, ds.frequency, ds.stage, COALESCE(ds.is_pathognomonic, 0) AS is_pathognomonic
         FROM disease_symptom ds
         JOIN symptoms s ON ds.symptom_id = s.id
         WHERE ds.disease_id = ?
         ORDER BY ds.frequency DESC"
    )
    .bind(&disease_id)
    .fetch_all(&*pool)
    .await
    .map(|rows| {
        rows.iter().map(|r| {
            let symptom = Symptom {
                id: r.get("id"),
                name_zh: r.get("name_zh"),
                name_en: r.get("name_en"),
                definition: r.get("definition"),
                species_notes: r.get("species_notes"),
                physiological_basis: r.get("physiological_basis"),
            };
            let frequency: String = r.get("frequency");
            let stage: String = r.get("stage");
            let is_pathognomonic: i64 = r.get("is_pathognomonic");
            (symptom, frequency, stage, is_pathognomonic)
        }).collect()
    })
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_disease_ddx(
    pool: tauri::State<'_, DbPool>,
    disease_id: String,
) -> Result<Vec<(Disease, String)>, String> {
    sqlx::query(
        "SELECT d.*, dd.distinguishing_feature FROM disease_ddx dd
         JOIN diseases d ON dd.ddx_id = d.id
         WHERE dd.disease_id = ?"
    )
    .bind(&disease_id)
    .fetch_all(&*pool)
    .await
    .map(|rows| {
        rows.iter().map(|r| {
            (row_to_disease(r), r.get::<String, _>("distinguishing_feature"))
        }).collect()
    })
    .map_err(|e| e.to_string())
}

fn row_to_drug(r: &sqlx::sqlite::SqliteRow) -> Drug {
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

#[tauri::command]
pub async fn get_disease_treatments(
    pool: tauri::State<'_, DbPool>,
    disease_id: String,
) -> Result<Vec<(Drug, String, String, String)>, String> {
    sqlx::query(
        "SELECT dr.*, dt.line, dt.species, dt.notes FROM disease_treatment dt
         JOIN drugs dr ON dt.drug_id = dr.id
         WHERE dt.disease_id = ?
         ORDER BY dt.line"
    )
    .bind(&disease_id)
    .fetch_all(&*pool)
    .await
    .map(|rows| {
        rows.iter().map(|r| {
            (
                row_to_drug(r),
                r.get::<String, _>("line"),
                r.get::<String, _>("species"),
                r.get::<String, _>("notes"),
            )
        }).collect()
    })
    .map_err(|e| e.to_string())
}

fn row_to_test(r: &sqlx::sqlite::SqliteRow) -> DiagnosticTest {
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

#[tauri::command]
pub async fn get_disease_diagnostics(
    pool: tauri::State<'_, DbPool>,
    disease_id: String,
) -> Result<Vec<(DiagnosticTest, String, String, String, String)>, String> {
    sqlx::query(
        "SELECT t.*, dd.purpose, dd.evidence_level, dd.species, dd.expected_result
         FROM disease_diagnostic dd
         JOIN diagnostic_tests t ON dd.test_id = t.id
         WHERE dd.disease_id = ?"
    )
    .bind(&disease_id)
    .fetch_all(&*pool)
    .await
    .map(|rows| {
        rows.iter().map(|r| {
            (
                row_to_test(r),
                r.get::<String, _>("purpose"),
                r.get::<String, _>("evidence_level"),
                r.get::<String, _>("species"),
                r.get::<String, _>("expected_result"),
            )
        }).collect()
    })
    .map_err(|e| e.to_string())
}

// ===== 疾病对比 =====

#[derive(serde::Serialize)]
pub struct DiseaseCompareView {
    pub disease: Disease,
    pub symptoms: Vec<(Symptom, String, String, i64)>,
    pub treatments: Vec<(Drug, String, String, String)>,
    pub diagnostics: Vec<(DiagnosticTest, String, String, String, String)>,
    pub ddx: Vec<(Disease, String)>,
}

#[tauri::command]
pub async fn get_disease_compare(
    pool: tauri::State<'_, DbPool>,
    disease_ids: Vec<String>,
) -> Result<Vec<DiseaseCompareView>, String> {
    let mut results = Vec::new();

    for did in &disease_ids {
        let disease = sqlx::query_as::<_, Disease>("SELECT * FROM diseases WHERE id = ?")
            .bind(did)
            .fetch_optional(&*pool)
            .await
            .map_err(|e| e.to_string())?;

        let Some(disease) = disease else { continue; };

        let symptoms = sqlx::query(
            "SELECT s.*, ds.frequency, ds.stage, COALESCE(ds.is_pathognomonic, 0) AS is_pathognomonic
             FROM disease_symptom ds
             JOIN symptoms s ON ds.symptom_id = s.id
             WHERE ds.disease_id = ?
             ORDER BY ds.frequency DESC"
        )
        .bind(did)
        .fetch_all(&*pool)
        .await
        .map(|rows| {
            rows.iter().map(|r| {
                let symptom = Symptom {
                    id: r.get("id"),
                    name_zh: r.get("name_zh"),
                    name_en: r.get("name_en"),
                    definition: r.get("definition"),
                    species_notes: r.get("species_notes"),
                    physiological_basis: r.get("physiological_basis"),
                };
                let frequency: String = r.get("frequency");
                let stage: String = r.get("stage");
                let is_pathognomonic: i64 = r.get("is_pathognomonic");
                (symptom, frequency, stage, is_pathognomonic)
            }).collect()
        })
        .unwrap_or_default();

        let treatments = sqlx::query(
            "SELECT dr.*, dt.line, dt.species, dt.notes FROM disease_treatment dt
             JOIN drugs dr ON dt.drug_id = dr.id
             WHERE dt.disease_id = ?
             ORDER BY dt.line"
        )
        .bind(did)
        .fetch_all(&*pool)
        .await
        .map(|rows| {
            rows.iter().map(|r| {
                (
                    row_to_drug(r),
                    r.get::<String, _>("line"),
                    r.get::<String, _>("species"),
                    r.get::<String, _>("notes"),
                )
            }).collect()
        })
        .unwrap_or_default();

        let diagnostics = sqlx::query(
            "SELECT t.*, dd.purpose, dd.evidence_level, dd.species, dd.expected_result
             FROM disease_diagnostic dd
             JOIN diagnostic_tests t ON dd.test_id = t.id
             WHERE dd.disease_id = ?"
        )
        .bind(did)
        .fetch_all(&*pool)
        .await
        .map(|rows| {
            rows.iter().map(|r| {
                (
                    row_to_test(r),
                    r.get::<String, _>("purpose"),
                    r.get::<String, _>("evidence_level"),
                    r.get::<String, _>("species"),
                    r.get::<String, _>("expected_result"),
                )
            }).collect()
        })
        .unwrap_or_default();

        let ddx = sqlx::query(
            "SELECT d.*, dd.distinguishing_feature FROM disease_ddx dd
             JOIN diseases d ON dd.ddx_id = d.id
             WHERE dd.disease_id = ?"
        )
        .bind(did)
        .fetch_all(&*pool)
        .await
        .map(|rows| {
            rows.iter().map(|r| {
                (row_to_disease(r), r.get::<String, _>("distinguishing_feature"))
            }).collect()
        })
        .unwrap_or_default();

        results.push(DiseaseCompareView {
            disease,
            symptoms,
            treatments,
            diagnostics,
            ddx,
        });
    }

    Ok(results)
}

// ===== 症状 =====

#[tauri::command]
pub async fn get_symptoms(
    pool: tauri::State<'_, DbPool>,
) -> Result<Vec<Symptom>, String> {
    sqlx::query_as::<_, Symptom>("SELECT * FROM symptoms ORDER BY name_zh")
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_symptom_by_id(
    pool: tauri::State<'_, DbPool>,
    id: String,
) -> Result<Option<Symptom>, String> {
    sqlx::query_as::<_, Symptom>("SELECT * FROM symptoms WHERE id = ?")
        .bind(&id)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_diseases_by_symptom(
    pool: tauri::State<'_, DbPool>,
    symptom_id: String,
    species: Option<String>,
) -> Result<Vec<DiseaseWithSymptom>, String> {
    let mut query = String::from(
        "SELECT d.*, ds.frequency, ds.stage, COALESCE(ds.is_pathognomonic, 0) AS is_pathognomonic
         FROM disease_symptom ds
         JOIN diseases d ON ds.disease_id = d.id
         WHERE ds.symptom_id = ?"
    );
    if species.is_some() {
        query.push_str(" AND d.species LIKE ?");
    }
    query.push_str(" ORDER BY ds.frequency DESC, d.name_zh");

    let mut q = sqlx::query(&query);
    q = q.bind(&symptom_id);
    if let Some(s) = &species {
        q = q.bind(format!("%{}%", s));
    }

    q.fetch_all(&*pool)
        .await
        .map(|rows| {
            rows.iter()
                .map(|r| DiseaseWithSymptom {
                    disease: row_to_disease(r),
                    frequency: r.get::<String, _>("frequency"),
                    stage: r.get::<String, _>("stage"),
                    is_pathognomonic: r.get::<i64, _>("is_pathognomonic"),
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

// ===== 药物 =====

#[tauri::command]
pub async fn get_drugs(
    pool: tauri::State<'_, DbPool>,
    drug_class: Option<String>,
) -> Result<Vec<Drug>, String> {
    let drug_class = drug_class.unwrap_or_default();
    let has_class = !drug_class.is_empty();

    let mut query = String::from("SELECT * FROM drugs WHERE 1=1");
    if has_class { query.push_str(" AND drug_class LIKE ?"); }
    query.push_str(" ORDER BY name_zh");

    let mut q = sqlx::query_as::<_, Drug>(&query);
    if has_class { q = q.bind(format!("%{}%", drug_class)); }

    q.fetch_all(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_drug_by_id(
    pool: tauri::State<'_, DbPool>,
    id: String,
) -> Result<Option<Drug>, String> {
    sqlx::query_as::<_, Drug>("SELECT * FROM drugs WHERE id = ?")
        .bind(&id)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| e.to_string())
}

// ===== 检查项目 =====

#[tauri::command]
pub async fn get_tests(
    pool: tauri::State<'_, DbPool>,
) -> Result<Vec<DiagnosticTest>, String> {
    sqlx::query_as::<_, DiagnosticTest>("SELECT * FROM diagnostic_tests ORDER BY name_zh")
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_test_by_id(
    pool: tauri::State<'_, DbPool>,
    id: String,
) -> Result<Option<DiagnosticTest>, String> {
    sqlx::query_as::<_, DiagnosticTest>("SELECT * FROM diagnostic_tests WHERE id = ?")
        .bind(&id)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| e.to_string())
}

// ===== 病例 =====

#[tauri::command]
pub async fn get_case_diseases(
    pool: tauri::State<'_, DbPool>,
    case_id: String,
) -> Result<Vec<Disease>, String> {
    sqlx::query_as::<_, Disease>(
        "SELECT d.* FROM diseases d
         INNER JOIN case_disease cd ON cd.disease_id = d.id
         WHERE cd.case_id = ?
         ORDER BY d.name_zh"
    )
    .bind(&case_id)
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_cases(
    pool: tauri::State<'_, DbPool>,
    species: Option<String>,
    difficulty: Option<String>,
) -> Result<Vec<Case>, String> {
    let species = species.unwrap_or_default();
    let difficulty = difficulty.unwrap_or_default();
    let has_species = !species.is_empty();
    let has_difficulty = !difficulty.is_empty();

    let mut query = String::from("SELECT * FROM cases WHERE 1=1");
    if has_species { query.push_str(" AND species = ?"); }
    if has_difficulty { query.push_str(" AND difficulty = ?"); }
    query.push_str(" ORDER BY title");

    let mut q = sqlx::query_as::<_, Case>(&query);
    if has_species { q = q.bind(species); }
    if has_difficulty { q = q.bind(difficulty); }

    q.fetch_all(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_case_by_id(
    pool: tauri::State<'_, DbPool>,
    id: String,
) -> Result<Option<Case>, String> {
    sqlx::query_as::<_, Case>("SELECT * FROM cases WHERE id = ?")
        .bind(&id)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| e.to_string())
}
