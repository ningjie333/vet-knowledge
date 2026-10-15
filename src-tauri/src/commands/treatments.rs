use crate::db::models::row_from_treatment;
use crate::db::sql_builder::tags_subquery;
use crate::db::{models::*, DbPool};
use sqlx::Row;

// ===== 治疗 =====

#[tauri::command]
pub async fn get_treatments(
    pool: tauri::State<'_, DbPool>,
    therapy_type: Option<String>,
) -> Result<Vec<Treatment>, String> {
    let base = format!(
        "SELECT t.*, {} FROM treatments t",
        tags_subquery("t", "treatment")
    );
    let sql = crate::db::sql_builder::SqlBuilder::select(base)
        .optional_where(
            therapy_type
                .as_ref()
                .filter(|t| !t.is_empty())
                .map(|_| "therapy_type = ?"),
        )
        .order_by("name_zh")
        .build();

    let mut q = sqlx::query(&sql);
    if let Some(t) = &therapy_type {
        if !t.is_empty() {
            q = q.bind(t);
        }
    }

    let rows = q.fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    Ok(rows.iter().map(row_from_treatment).collect())
}

#[tauri::command]
pub async fn get_treatment_by_id(
    pool: tauri::State<'_, DbPool>,
    id: String,
) -> Result<Option<Treatment>, String> {
    let sql = format!(
        "SELECT t.*, {} FROM treatments t WHERE t.id = ?",
        tags_subquery("t", "treatment")
    );
    let row = sqlx::query(&sql)
        .bind(&id)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(row.as_ref().map(row_from_treatment))
}

#[tauri::command]
pub async fn get_disease_treatment_map(
    pool: tauri::State<'_, DbPool>,
    disease_id: String,
) -> Result<Vec<(Treatment, String, String, String)>, String> {
    let sql = format!(
        "SELECT t.*, {}, dtm.line, dtm.species, dtm.notes FROM disease_treatment_map dtm
         JOIN treatments t ON dtm.treatment_id = t.id
         WHERE dtm.disease_id = ?
         ORDER BY dtm.line",
        tags_subquery("t", "treatment")
    );
    let rows = sqlx::query(&sql)
        .bind(&disease_id)
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows
        .iter()
        .map(|r| {
            let treatment = row_from_treatment(r);
            let line: String = r.get("line");
            let species: String = r.get("species");
            let notes: String = r.get("notes");
            (treatment, line, species, notes)
        })
        .collect())
}

// ===== 标签 =====

#[tauri::command]
pub async fn get_tags(
    pool: tauri::State<'_, DbPool>,
    tag_group: Option<String>,
) -> Result<Vec<Tag>, String> {
    let sql = crate::db::sql_builder::SqlBuilder::select("SELECT * FROM tags")
        .optional_where(
            tag_group
                .as_ref()
                .filter(|g| !g.is_empty())
                .map(|_| "tag_group = ?"),
        )
        .order_by("tag_group, name_zh")
        .build();

    let mut q = sqlx::query_as::<_, Tag>(&sql);
    if let Some(g) = &tag_group {
        if !g.is_empty() {
            q = q.bind(g);
        }
    }

    q.fetch_all(&*pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_entity_tags(
    pool: tauri::State<'_, DbPool>,
    entity_type: String,
    entity_id: String,
) -> Result<Vec<Tag>, String> {
    if !crate::db::entity_types::is_valid_entity_type(&entity_type) {
        return Err(format!("无效的 entity_type: {}", entity_type));
    }

    sqlx::query_as::<_, Tag>(
        "SELECT t.* FROM entity_tags et
         JOIN tags t ON et.tag_id = t.id
         WHERE et.entity_type = ? AND et.entity_id = ?
         ORDER BY t.tag_group, t.name_zh",
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
         WHERE tag_id = ? AND entity_type = ?",
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
    if !crate::db::entity_types::is_valid_entity_type(&entity_type) {
        return Err(format!("无效的 entity_type: {}", entity_type));
    }
    sqlx::query(
        "INSERT OR IGNORE INTO entity_tags (entity_type, entity_id, tag_id)
         VALUES (?, ?, ?)",
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
    if !crate::db::entity_types::is_valid_entity_type(&entity_type) {
        return Err(format!("无效的 entity_type: {}", entity_type));
    }
    sqlx::query(
        "DELETE FROM entity_tags
         WHERE entity_type = ? AND entity_id = ? AND tag_id = ?",
    )
    .bind(&entity_type)
    .bind(&entity_id)
    .bind(&tag_id)
    .execute(&*pool)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}
