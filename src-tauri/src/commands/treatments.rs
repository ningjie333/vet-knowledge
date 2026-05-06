use crate::db::{DbPool, models::*};
use sqlx::Row;

// ===== 治疗 =====

#[tauri::command]
pub async fn get_treatments(
    pool: tauri::State<'_, DbPool>,
    therapy_type: Option<String>,
) -> Result<Vec<Treatment>, String> {
    let mut query = String::from("SELECT * FROM treatments WHERE 1=1");
    if therapy_type.is_some() {
        query.push_str(" AND therapy_type = ?");
    }
    query.push_str(" ORDER BY name_zh");

    let mut q = sqlx::query_as::<_, Treatment>(&query);
    if let Some(t) = therapy_type {
        q = q.bind(t);
    }

    q.fetch_all(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_treatment_by_id(
    pool: tauri::State<'_, DbPool>,
    id: String,
) -> Result<Option<Treatment>, String> {
    sqlx::query_as::<_, Treatment>("SELECT * FROM treatments WHERE id = ?")
        .bind(&id)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_disease_treatment_map(
    pool: tauri::State<'_, DbPool>,
    disease_id: String,
) -> Result<Vec<(Treatment, String, String, String)>, String> {
    sqlx::query(
        "SELECT t.*, dtm.line, dtm.species, dtm.notes FROM disease_treatment_map dtm
         JOIN treatments t ON dtm.treatment_id = t.id
         WHERE dtm.disease_id = ?
         ORDER BY dtm.line"
    )
    .bind(&disease_id)
    .fetch_all(&*pool)
    .await
    .map(|rows| {
        rows.iter().map(|r| {
            let treatment = Treatment {
                id: r.get("id"),
                name_zh: r.get("name_zh"),
                name_en: r.get("name_en"),
                therapy_type: r.get("therapy_type"),
                principle: r.get("principle"),
                procedure_text: r.get("procedure_text"),
                physiological_basis: r.get("physiological_basis"),
                prognosis_assessment: r.get("prognosis_assessment"),
                created_at: r.get("created_at"),
            };
            let line: String = r.get("line");
            let species: String = r.get("species");
            let notes: String = r.get("notes");
            (treatment, line, species, notes)
        }).collect()
    })
    .map_err(|e| e.to_string())
}

// ===== 标签 =====

#[tauri::command]
pub async fn get_tags(
    pool: tauri::State<'_, DbPool>,
    tag_group: Option<String>,
) -> Result<Vec<Tag>, String> {
    let mut query = String::from("SELECT * FROM tags WHERE 1=1");
    if tag_group.is_some() {
        query.push_str(" AND tag_group = ?");
    }
    query.push_str(" ORDER BY tag_group, name_zh");

    let mut q = sqlx::query_as::<_, Tag>(&query);
    if let Some(g) = tag_group {
        q = q.bind(g);
    }

    q.fetch_all(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_entity_tags(
    pool: tauri::State<'_, DbPool>,
    entity_type: String,
    entity_id: String,
) -> Result<Vec<Tag>, String> {
    sqlx::query_as::<_, Tag>(
        "SELECT t.* FROM entity_tags et
         JOIN tags t ON et.tag_id = t.id
         WHERE et.entity_type = ? AND et.entity_id = ?
         ORDER BY t.tag_group, t.name_zh"
    )
    .bind(&entity_type)
    .bind(&entity_id)
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_entities_by_tag(
    pool: tauri::State<'_, DbPool>,
    tag_id: String,
    entity_type: String,
) -> Result<Vec<String>, String> {
    sqlx::query_scalar::<_, String>(
        "SELECT entity_id FROM entity_tags
         WHERE tag_id = ? AND entity_type = ?"
    )
    .bind(&tag_id)
    .bind(&entity_type)
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_entity_tag(
    pool: tauri::State<'_, DbPool>,
    entity_type: String,
    entity_id: String,
    tag_id: String,
) -> Result<(), String> {
    sqlx::query(
        "INSERT OR IGNORE INTO entity_tags (entity_type, entity_id, tag_id)
         VALUES (?, ?, ?)"
    )
    .bind(&entity_type)
    .bind(&entity_id)
    .bind(&tag_id)
    .execute(&*pool)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_entity_tag(
    pool: tauri::State<'_, DbPool>,
    entity_type: String,
    entity_id: String,
    tag_id: String,
) -> Result<(), String> {
    sqlx::query(
        "DELETE FROM entity_tags
         WHERE entity_type = ? AND entity_id = ? AND tag_id = ?"
    )
    .bind(&entity_type)
    .bind(&entity_id)
    .bind(&tag_id)
    .execute(&*pool)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}
