use crate::db::{DbPool, models::*};
use crate::db::models::{row_from_disease, row_from_drug, row_from_symptom, row_from_test};
use crate::db::sql_builder::tags_subquery;
use sqlx::Row;

// 标签链路约定：tags_subquery() 在 SQL 侧把 entity_tags → tags.id 聚合成 JSON 文本，
// 由 models::row_from_*() 解析成 Vec<String> 后返回前端（前端 tags: string[]，元素是标签 id，
// 供页面回查 get_tags() 取 color / tag_group）。
// 凡是返回 Disease / Symptom / Drug / Treatment 的查询都要带上对应的 tags 子查询，
// 否则前端的标签区域就是空白。

// ===== 疾病 =====

#[tauri::command]
pub async fn get_diseases(
    pool: tauri::State<'_, DbPool>,
    species: Option<String>,
    category: Option<String>,
) -> Result<Vec<Disease>, String> {
    let base = format!("SELECT d.*, {} FROM diseases d", tags_subquery("d", "disease"));
    let sql = crate::db::sql_builder::SqlBuilder::select(base)
        .optional_where(species.as_ref().filter(|s| !s.is_empty()).map(|_| "species LIKE ?"))
        .optional_where(category.as_ref().filter(|c| !c.is_empty()).map(|_| "category LIKE ?"))
        .order_by("name_zh")
        .build();

    let mut q = sqlx::query(&sql);
    if let Some(s) = &species { if !s.is_empty() { q = q.bind(format!("%{}%", s)); } }
    if let Some(c) = &category { if !c.is_empty() { q = q.bind(format!("%{}%", c)); } }

    let rows = q.fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    Ok(rows.iter().map(row_from_disease).collect())
}

#[tauri::command]
pub async fn get_disease_by_id(
    pool: tauri::State<'_, DbPool>,
    id: String,
) -> Result<Option<Disease>, String> {
    let sql = format!(
        "SELECT d.*, {} FROM diseases d WHERE d.id = ?",
        tags_subquery("d", "disease")
    );
    let row = sqlx::query(&sql)
        .bind(&id)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(row.as_ref().map(row_from_disease))
}

#[tauri::command]
pub async fn get_disease_symptoms(
    pool: tauri::State<'_, DbPool>,
    disease_id: String,
) -> Result<Vec<(Symptom, String, String, i64)>, String> {
    let sql = format!(
        "SELECT s.*, {}, ds.frequency, ds.stage, COALESCE(ds.is_pathognomonic, 0) AS is_pathognomonic
         FROM disease_symptom ds
         JOIN symptoms s ON ds.symptom_id = s.id
         WHERE ds.disease_id = ?
         ORDER BY ds.frequency DESC",
        tags_subquery("s", "symptom")
    );
    let rows = sqlx::query(&sql)
        .bind(&disease_id)
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows.iter().map(|r| {
        let symptom = row_from_symptom(r);
        let frequency: String = r.get("frequency");
        let stage: String = r.get("stage");
        let is_pathognomonic: i64 = r.get("is_pathognomonic");
        (symptom, frequency, stage, is_pathognomonic)
    }).collect())
}

#[tauri::command]
pub async fn get_disease_ddx(
    pool: tauri::State<'_, DbPool>,
    disease_id: String,
) -> Result<Vec<(Disease, String)>, String> {
    let sql = format!(
        "SELECT d.*, {}, dd.distinguishing_feature FROM disease_ddx dd
         JOIN diseases d ON dd.ddx_id = d.id
         WHERE dd.disease_id = ?",
        tags_subquery("d", "disease")
    );
    let rows = sqlx::query(&sql)
        .bind(&disease_id)
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows.iter().map(|r| {
        (row_from_disease(r), r.get::<String, _>("distinguishing_feature"))
    }).collect())
}

#[tauri::command]
pub async fn get_disease_treatments(
    pool: tauri::State<'_, DbPool>,
    disease_id: String,
) -> Result<Vec<(Drug, String, String, String)>, String> {
    let sql = format!(
        "SELECT dr.*, {}, dt.line, dt.species, dt.notes FROM disease_treatment dt
         JOIN drugs dr ON dt.drug_id = dr.id
         WHERE dt.disease_id = ?
         ORDER BY dt.line",
        tags_subquery("dr", "drug")
    );
    let rows = sqlx::query(&sql)
        .bind(&disease_id)
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows.iter().map(|r| {
        (
            row_from_drug(r),
            r.get::<String, _>("line"),
            r.get::<String, _>("species"),
            r.get::<String, _>("notes"),
        )
    }).collect())
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
                row_from_test(r),
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
    if disease_ids.is_empty() {
        return Ok(Vec::new());
    }

    let ph = crate::db::sql_builder::placeholders(disease_ids.len());

    // 1. 批量获取疾病（保持输入顺序）
    let sql = format!(
        "SELECT d.*, {} FROM diseases d WHERE d.id IN ({})",
        tags_subquery("d", "disease"), ph
    );
    let mut q = sqlx::query(&sql);
    for did in &disease_ids { q = q.bind(did); }
    let diseases: Vec<Disease> = q
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?
        .iter()
        .map(row_from_disease)
        .collect();

    // 2. 批量获取症状、治疗、诊断、鉴别（4 条查询，Rust 端按 disease_id 分组）
    let group_symptoms = |rows: Vec<sqlx::sqlite::SqliteRow>| {
        let mut map: std::collections::HashMap<String, Vec<(Symptom, String, String, i64)>> = std::collections::HashMap::new();
        for r in &rows {
            let did: String = r.get("disease_id");
            let entry = map.entry(did).or_default();
            entry.push((row_from_symptom(r), r.get("frequency"), r.get("stage"), r.get("is_pathognomonic")));
        }
        map
    };

    let group_drugs = |rows: Vec<sqlx::sqlite::SqliteRow>| {
        let mut map: std::collections::HashMap<String, Vec<(Drug, String, String, String)>> = std::collections::HashMap::new();
        for r in &rows {
            let did: String = r.get("disease_id");
            let entry = map.entry(did).or_default();
            entry.push((row_from_drug(r), r.get("line"), r.get("species"), r.get("notes")));
        }
        map
    };

    let group_tests = |rows: Vec<sqlx::sqlite::SqliteRow>| {
        let mut map: std::collections::HashMap<String, Vec<(DiagnosticTest, String, String, String, String)>> = std::collections::HashMap::new();
        for r in &rows {
            let did: String = r.get("disease_id");
            let entry = map.entry(did).or_default();
            entry.push((row_from_test(r), r.get("purpose"), r.get("evidence_level"), r.get("species"), r.get("expected_result")));
        }
        map
    };

    let group_ddx = |rows: Vec<sqlx::sqlite::SqliteRow>| {
        let mut map: std::collections::HashMap<String, Vec<(Disease, String)>> = std::collections::HashMap::new();
        for r in &rows {
            let did: String = r.get("disease_id");
            let entry = map.entry(did).or_default();
            entry.push((row_from_disease(r), r.get("distinguishing_feature")));
        }
        map
    };

    // 症状
    let sql = format!(
        "SELECT ds.disease_id, s.*, {}, ds.frequency, ds.stage, COALESCE(ds.is_pathognomonic, 0) AS is_pathognomonic
         FROM disease_symptom ds JOIN symptoms s ON ds.symptom_id = s.id
         WHERE ds.disease_id IN ({})", tags_subquery("s", "symptom"), ph);
    let mut q = sqlx::query(&sql);
    for did in &disease_ids { q = q.bind(did); }
    let symptoms_map = group_symptoms(q.fetch_all(&*pool).await.map_err(|e| e.to_string())?);

    // 治疗
    let sql = format!(
        "SELECT dt.disease_id, dr.*, {}, dt.line, dt.species, dt.notes
         FROM disease_treatment dt JOIN drugs dr ON dt.drug_id = dr.id
         WHERE dt.disease_id IN ({})", tags_subquery("dr", "drug"), ph);
    let mut q = sqlx::query(&sql);
    for did in &disease_ids { q = q.bind(did); }
    let treatments_map = group_drugs(q.fetch_all(&*pool).await.map_err(|e| e.to_string())?);

    // 诊断
    let sql = format!(
        "SELECT dd.disease_id, t.*, dd.purpose, dd.evidence_level, dd.species, dd.expected_result
         FROM disease_diagnostic dd JOIN diagnostic_tests t ON dd.test_id = t.id
         WHERE dd.disease_id IN ({})", ph);
    let mut q = sqlx::query(&sql);
    for did in &disease_ids { q = q.bind(did); }
    let diagnostics_map = group_tests(q.fetch_all(&*pool).await.map_err(|e| e.to_string())?);

    // 鉴别
    let sql = format!(
        "SELECT dd.disease_id, d.*, {}, dd.distinguishing_feature
         FROM disease_ddx dd JOIN diseases d ON dd.ddx_id = d.id
         WHERE dd.disease_id IN ({})", tags_subquery("d", "disease"), ph);
    let mut q = sqlx::query(&sql);
    for did in &disease_ids { q = q.bind(did); }
    let ddx_map = group_ddx(q.fetch_all(&*pool).await.map_err(|e| e.to_string())?);

    // 3. 组装结果
    let results: Vec<DiseaseCompareView> = diseases.iter().map(|disease| {
        let did = &disease.id;
        DiseaseCompareView {
            disease: disease.clone(),
            symptoms: symptoms_map.get(did).cloned().unwrap_or_default(),
            treatments: treatments_map.get(did).cloned().unwrap_or_default(),
            diagnostics: diagnostics_map.get(did).cloned().unwrap_or_default(),
            ddx: ddx_map.get(did).cloned().unwrap_or_default(),
        }
    }).collect();

    Ok(results)
}

// ===== 症状 =====

#[tauri::command]
pub async fn get_symptoms(
    pool: tauri::State<'_, DbPool>,
) -> Result<Vec<Symptom>, String> {
    let sql = format!(
        "SELECT s.*, {} FROM symptoms s ORDER BY s.name_zh",
        tags_subquery("s", "symptom")
    );
    let rows = sqlx::query(&sql)
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows.iter().map(row_from_symptom).collect())
}

#[tauri::command]
pub async fn get_symptom_by_id(
    pool: tauri::State<'_, DbPool>,
    id: String,
) -> Result<Option<Symptom>, String> {
    let sql = format!(
        "SELECT s.*, {} FROM symptoms s WHERE s.id = ?",
        tags_subquery("s", "symptom")
    );
    let row = sqlx::query(&sql)
        .bind(&id)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(row.as_ref().map(row_from_symptom))
}

#[tauri::command]
pub async fn get_diseases_by_symptom(
    pool: tauri::State<'_, DbPool>,
    symptom_id: String,
    species: Option<String>,
) -> Result<Vec<DiseaseWithSymptom>, String> {
    let mut query = format!(
        "SELECT d.*, {}, ds.frequency, ds.stage, COALESCE(ds.is_pathognomonic, 0) AS is_pathognomonic
         FROM disease_symptom ds
         JOIN diseases d ON ds.disease_id = d.id
         WHERE ds.symptom_id = ?",
        tags_subquery("d", "disease")
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
                    disease: row_from_disease(r),
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
    let base = format!("SELECT dr.*, {} FROM drugs dr", tags_subquery("dr", "drug"));
    let sql = crate::db::sql_builder::SqlBuilder::select(base)
        .optional_where(drug_class.as_ref().filter(|c| !c.is_empty()).map(|_| "drug_class LIKE ?"))
        .order_by("name_zh")
        .build();

    let mut q = sqlx::query(&sql);
    if let Some(c) = &drug_class { if !c.is_empty() { q = q.bind(format!("%{}%", c)); } }

    let rows = q.fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    Ok(rows.iter().map(row_from_drug).collect())
}

#[tauri::command]
pub async fn get_drug_by_id(
    pool: tauri::State<'_, DbPool>,
    id: String,
) -> Result<Option<Drug>, String> {
    let sql = format!(
        "SELECT dr.*, {} FROM drugs dr WHERE dr.id = ?",
        tags_subquery("dr", "drug")
    );
    let row = sqlx::query(&sql)
        .bind(&id)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(row.as_ref().map(row_from_drug))
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
    let sql = format!(
        "SELECT d.*, {} FROM diseases d
         INNER JOIN case_disease cd ON cd.disease_id = d.id
         WHERE cd.case_id = ?
         ORDER BY d.name_zh",
        tags_subquery("d", "disease")
    );
    let rows = sqlx::query(&sql)
        .bind(&case_id)
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows.iter().map(row_from_disease).collect())
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

// ===== 药物相互作用 =====

#[derive(Debug, serde::Serialize)]
pub struct DrugInteraction {
    pub drug_id: String,      // 对方药物 ID
    pub drug_name: String,    // 对方药物名
    pub severity: String,     // major / moderate / mild
    pub description: String,
}

/// 获取与指定药物有相互作用的所有药物
#[tauri::command]
pub async fn get_drug_interactions(
    pool: tauri::State<'_, DbPool>,
    drug_id: String,
) -> Result<Vec<DrugInteraction>, String> {
    sqlx::query(
        "SELECT
            CASE WHEN drug_a_id = ? THEN drug_b_id ELSE drug_a_id END AS drug_id,
            d.name_zh AS drug_name,
            di.severity,
            di.description
         FROM drug_interactions di
         JOIN drugs d ON d.id = CASE WHEN di.drug_a_id = ? THEN di.drug_b_id ELSE di.drug_a_id END
         WHERE di.drug_a_id = ? OR di.drug_b_id = ?
         ORDER BY CASE di.severity
             WHEN 'major' THEN 3
             WHEN 'moderate' THEN 2
             WHEN 'mild' THEN 1
         END DESC, d.name_zh"
    )
    .bind(&drug_id)
    .bind(&drug_id)
    .bind(&drug_id)
    .bind(&drug_id)
    .fetch_all(&*pool)
    .await
    .map(|rows| {
        rows.iter().map(|r| DrugInteraction {
            drug_id: r.get("drug_id"),
            drug_name: r.get("drug_name"),
            severity: r.get("severity"),
            description: r.get("description"),
        }).collect()
    })
    .map_err(|e| e.to_string())
}
