use crate::db::DbPool;
use serde::Serialize;
use sqlx::Row;

#[derive(Serialize)]
pub struct SearchResult {
    pub entity_type: String,
    pub entity_id: String,
    pub title: String,
    pub snippet: String,
    pub relevance: f64,
}

#[tauri::command]
pub async fn full_text_search(
    pool: tauri::State<'_, DbPool>,
    query: String,
    limit: Option<i64>,
) -> Result<Vec<SearchResult>, String> {
    let limit = limit.unwrap_or(20);
    let q = format!("%{}%", query.trim());

    let mut results = Vec::new();

    // 搜索疾病
    let disease_rows = sqlx::query(
        "SELECT id, name_zh, name_en, overview FROM diseases
         WHERE name_zh LIKE ? OR name_en LIKE ? OR overview LIKE ?
         LIMIT ?"
    )
    .bind(&q).bind(&q).bind(&q)
    .bind(limit)
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    for row in &disease_rows {
        let name_zh: String = row.get("name_zh");
        let name_en: Option<String> = row.get("name_en");
        let overview: Option<String> = row.get("overview");
        let title = if let Some(en) = name_en {
            format!("{} / {}", name_zh, en)
        } else {
            name_zh.clone()
        };
        let snippet = overview.unwrap_or_default();
        let relevance = if name_zh.contains(query.trim()) { 1.0 } else { 0.7 };
        results.push(SearchResult {
            entity_type: "disease".to_string(),
            entity_id: row.get("id"),
            title,
            snippet,
            relevance,
        });
    }

    // 搜索症状
    let symptom_rows = sqlx::query(
        "SELECT id, name_zh, name_en, definition FROM symptoms
         WHERE name_zh LIKE ? OR name_en LIKE ? OR definition LIKE ?
         LIMIT ?"
    )
    .bind(&q).bind(&q).bind(&q)
    .bind(limit)
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    for row in &symptom_rows {
        let name_zh: String = row.get("name_zh");
        let name_en: Option<String> = row.get("name_en");
        let definition: Option<String> = row.get("definition");
        let title = if let Some(en) = name_en {
            format!("{} / {}", name_zh, en)
        } else {
            name_zh.clone()
        };
        results.push(SearchResult {
            entity_type: "symptom".to_string(),
            entity_id: row.get("id"),
            title,
            snippet: definition.unwrap_or_default(),
            relevance: 0.8,
        });
    }

    // 搜索药物
    let drug_rows = sqlx::query(
        "SELECT id, name_zh, name_en, drug_class FROM drugs
         WHERE name_zh LIKE ? OR name_en LIKE ? OR drug_class LIKE ?
         LIMIT ?"
    )
    .bind(&q).bind(&q).bind(&q)
    .bind(limit)
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    for row in &drug_rows {
        let name_zh: String = row.get("name_zh");
        let name_en: Option<String> = row.get("name_en");
        let class: Option<String> = row.get("drug_class");
        let title = if let Some(en) = name_en {
            format!("{} / {}", name_zh, en)
        } else {
            name_zh.clone()
        };
        results.push(SearchResult {
            entity_type: "drug".to_string(),
            entity_id: row.get("id"),
            title,
            snippet: class.unwrap_or_default(),
            relevance: 0.6,
        });
    }

    // 搜索病例
    let case_rows = sqlx::query(
        "SELECT id, title, chief_complaint, diagnosis FROM cases
         WHERE title LIKE ? OR chief_complaint LIKE ? OR diagnosis LIKE ?
         LIMIT ?"
    )
    .bind(&q).bind(&q).bind(&q)
    .bind(limit)
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    for row in &case_rows {
        let title: String = row.get("title");
        let chief_complaint: Option<String> = row.get("chief_complaint");
        results.push(SearchResult {
            entity_type: "case".to_string(),
            entity_id: row.get("id"),
            title,
            snippet: chief_complaint.unwrap_or_default(),
            relevance: 0.75,
        });
    }

    // 按相关性排序
    results.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap());
    results.truncate(limit as usize);

    Ok(results)
}
