use crate::db::DbPool;
use crate::engine::{infer, DiagnosisInput, DiagnosisCandidate};
use sqlx::Row;

#[tauri::command]
pub async fn infer_diagnosis(
    pool: tauri::State<'_, DbPool>,
    symptoms: Vec<String>,
    species: String,
    age: Option<f64>,
    breed: Option<String>,
) -> Result<Vec<DiagnosisCandidate>, String> {
    let input = DiagnosisInput {
        symptoms: symptoms.clone(),
        species: species.clone(),
        age,
        breed,
    };

    let disease_list: Vec<(String, String)> = sqlx::query(
        "SELECT DISTINCT d.id, d.name_zh
         FROM diseases d
         INNER JOIN disease_symptom ds ON ds.disease_id = d.id
         WHERE d.species LIKE ?"
    )
    .bind(format!("%{}%", species))
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())?
    .iter()
    .map(|r| (
        r.get::<String, _>("id"),
        r.get::<String, _>("name_zh"),
    ))
    .collect();

    let mut all_disease_symptoms: std::collections::HashMap<String, Vec<(String, String, i64)>> = std::collections::HashMap::new();
    if !disease_list.is_empty() {
        let placeholders: Vec<String> = disease_list.iter().map(|_| "?".to_string()).collect();
        let query = format!(
            "SELECT ds.disease_id, s.name_zh, ds.frequency, COALESCE(ds.is_pathognomonic, 0) AS is_pathognomonic
             FROM disease_symptom ds
             JOIN symptoms s ON ds.symptom_id = s.id
             WHERE ds.disease_id IN ({})
             ORDER BY ds.disease_id",
            placeholders.join(",")
        );
        let mut q = sqlx::query(&query);
        for (did, _) in &disease_list {
            q = q.bind(did);
        }
        let rows = q.fetch_all(&*pool).await.map_err(|e| e.to_string())?;
        for r in &rows {
            let did: String = r.get("disease_id");
            let sym_name: String = r.get("name_zh");
            let freq: String = r.get("frequency");
            let is_patho: i64 = r.get("is_pathognomonic");
            all_disease_symptoms.entry(did).or_default().push((sym_name, freq, is_patho));
        }
    }

    let mut diagnostics: std::collections::HashMap<String, Vec<(String, String)>> = std::collections::HashMap::new();
    if !disease_list.is_empty() {
        let placeholders: Vec<String> = disease_list.iter().map(|_| "?".to_string()).collect();
        let query = format!(
            "SELECT dt.disease_id, dt.test_id, dt.purpose
             FROM disease_diagnostic dt
             WHERE dt.disease_id IN ({})
             ORDER BY dt.disease_id",
            placeholders.join(",")
        );
        let mut q = sqlx::query(&query);
        for (did, _) in &disease_list {
            q = q.bind(did);
        }
        let rows = q.fetch_all(&*pool).await.map_err(|e| e.to_string())?;
        for r in &rows {
            let did: String = r.get("disease_id");
            let test_id: String = r.get("test_id");
            let purpose: String = r.get("purpose");
            diagnostics.entry(did).or_default().push((test_id, purpose));
        }
    }

    let candidates = infer(&input, &disease_list, &all_disease_symptoms, &diagnostics);

    Ok(candidates)
}
