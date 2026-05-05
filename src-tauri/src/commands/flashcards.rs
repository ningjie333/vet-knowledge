use crate::db::DbPool;
use sqlx::Row;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Flashcard {
    pub id: String,
    pub front: String,
    pub back: String,
    pub card_type: String,
    pub entity_id: Option<String>,
    pub difficulty: f64,
    pub created_at: String,
    pub next_review: Option<String>,
    pub review_count: i64,
    pub ease_factor: f64,
}

#[derive(Debug, Serialize)]
pub struct ReviewStats {
    pub total_cards: i64,
    pub due_today: i64,
    pub reviewed_today: i64,
    pub mastered: i64,
}

/// 获取今日到期的闪卡（SM-2 调度）
#[tauri::command]
pub async fn get_due_flashcards(
    pool: tauri::State<'_, DbPool>,
    limit: Option<i64>,
) -> Result<Vec<Flashcard>, String> {
    let limit = limit.unwrap_or(20);
    let rows = sqlx::query(
        "SELECT f.id, f.front, f.back, f.card_type, f.entity_id, f.difficulty, f.created_at,
                fr.next_review,
                COALESCE(rev.review_count, 0) AS review_count,
                COALESCE(fr.ease_factor, 2.5) AS ease_factor
         FROM flashcards f
         LEFT JOIN (
             SELECT card_id, MAX(id) AS max_id
             FROM flashcard_reviews
             GROUP BY card_id
         ) latest ON latest.card_id = f.id
         LEFT JOIN flashcard_reviews fr ON fr.id = latest.max_id
         LEFT JOIN (
             SELECT card_id, COUNT(*) AS review_count
             FROM flashcard_reviews
             GROUP BY card_id
         ) rev ON rev.card_id = f.id
         WHERE fr.next_review IS NULL OR fr.next_review <= datetime('now')
         ORDER BY fr.next_review ASC
         LIMIT ?"
    )
    .bind(limit)
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows.iter().map(|r| Flashcard {
        id: r.get("id"),
        front: r.get("front"),
        back: r.get("back"),
        card_type: r.get("card_type"),
        entity_id: r.get("entity_id"),
        difficulty: r.get("difficulty"),
        created_at: r.get("created_at"),
        next_review: r.get("next_review"),
        review_count: r.get("review_count"),
        ease_factor: r.get("ease_factor"),
    }).collect())
}

/// 获取所有闪卡（管理用）
#[tauri::command]
pub async fn get_all_flashcards(
    pool: tauri::State<'_, DbPool>,
    card_type: Option<String>,
) -> Result<Vec<Flashcard>, String> {
    let mut query = String::from(
        "SELECT f.id, f.front, f.back, f.card_type, f.entity_id, f.difficulty, f.created_at,
                fr.next_review,
                COALESCE(rev.review_count, 0) AS review_count,
                COALESCE(fr.ease_factor, 2.5) AS ease_factor
         FROM flashcards f
         LEFT JOIN (
             SELECT card_id, MAX(id) AS max_id
             FROM flashcard_reviews
             GROUP BY card_id
         ) latest ON latest.card_id = f.id
         LEFT JOIN flashcard_reviews fr ON fr.id = latest.max_id
         LEFT JOIN (
             SELECT card_id, COUNT(*) AS review_count
             FROM flashcard_reviews
             GROUP BY card_id
         ) rev ON rev.card_id = f.id
         WHERE 1=1"
    );
    if card_type.is_some() {
        query.push_str(" AND f.card_type = ?");
    }
    query.push_str(" ORDER BY f.created_at DESC");

    let mut q = sqlx::query(&query);
    if let Some(ct) = &card_type {
        q = q.bind(ct);
    }

    let rows = q.fetch_all(&*pool).await.map_err(|e| e.to_string())?;
    Ok(rows.iter().map(|r| Flashcard {
        id: r.get("id"),
        front: r.get("front"),
        back: r.get("back"),
        card_type: r.get("card_type"),
        entity_id: r.get("entity_id"),
        difficulty: r.get("difficulty"),
        created_at: r.get("created_at"),
        next_review: r.get("next_review"),
        review_count: r.get("review_count"),
        ease_factor: r.get("ease_factor"),
    }).collect())
}

/// 从知识库自动生成闪卡（疾病、症状、药物）
#[tauri::command]
pub async fn generate_flashcards_from_knowledge(
    pool: tauri::State<'_, DbPool>,
    card_type: String,
) -> Result<i64, String> {
    let mut count = 0i64;

    match card_type.as_str() {
        "disease" => {
            // 疾病闪卡：正面=疾病名，背面=概述+紧急程度
            let rows = sqlx::query(
                "SELECT d.id, d.name_zh, d.overview, d.urgency_level, d.category
                 FROM diseases d
                 WHERE d.id NOT IN (SELECT entity_id FROM flashcards WHERE card_type = 'disease' AND entity_id IS NOT NULL)"
            ).fetch_all(&*pool).await.map_err(|e| e.to_string())?;

            for r in &rows {
                let id: String = r.get("id");
                let name: String = r.get("name_zh");
                let overview: Option<String> = r.get("overview");
                let urgency: Option<i64> = r.get("urgency_level");
                let category: Option<String> = r.get("category");
                let back = format!(
                    "分类：{}\n紧急程度：{}\n\n{}",
                    category.as_deref().unwrap_or("未知"),
                    urgency.map(|u| format!("{}/5", u)).unwrap_or_else(|| "未知".to_string()),
                    overview.as_deref().unwrap_or("暂无概述")
                );
                sqlx::query("INSERT OR IGNORE INTO flashcards (id, front, back, card_type, entity_id, difficulty) VALUES (?, ?, ?, 'disease', ?, 0.5)")
                    .bind(format!("fc_disease_{}", id))
                    .bind(&name)
                    .bind(&back)
                    .bind(&id)
                    .execute(&*pool).await.map_err(|e| e.to_string())?;
                count += 1;
            }
        }
        "symptom" => {
            // 症状闪卡：正面=症状名，背面=定义
            let rows = sqlx::query(
                "SELECT s.id, s.name_zh, s.definition
                 FROM symptoms s
                 WHERE s.id NOT IN (SELECT entity_id FROM flashcards WHERE card_type = 'symptom' AND entity_id IS NOT NULL)"
            ).fetch_all(&*pool).await.map_err(|e| e.to_string())?;

            for r in &rows {
                let id: String = r.get("id");
                let name: String = r.get("name_zh");
                let def: Option<String> = r.get("definition");
                let back = def.as_deref().unwrap_or("暂无定义");
                sqlx::query("INSERT OR IGNORE INTO flashcards (id, front, back, card_type, entity_id, difficulty) VALUES (?, ?, ?, 'symptom', ?, 0.3)")
                    .bind(format!("fc_symptom_{}", id))
                    .bind(&name)
                    .bind(back)
                    .bind(&id)
                    .execute(&*pool).await.map_err(|e| e.to_string())?;
                count += 1;
            }
        }
        "drug" => {
            // 药物闪卡：正面=药名，背面=适应症+注意事项
            let rows = sqlx::query(
                "SELECT dr.id, dr.name_zh, dr.indications, dr.contraindications, dr.side_effects, dr.drug_class
                 FROM drugs dr
                 WHERE dr.id NOT IN (SELECT entity_id FROM flashcards WHERE card_type = 'drug' AND entity_id IS NOT NULL)"
            ).fetch_all(&*pool).await.map_err(|e| e.to_string())?;

            for r in &rows {
                let id: String = r.get("id");
                let name: String = r.get("name_zh");
                let indications: Option<String> = r.get("indications");
                let contras: Option<String> = r.get("contraindications");
                let side: Option<String> = r.get("side_effects");
                let class: Option<String> = r.get("drug_class");
                let back = format!(
                    "药物分类：{}\n\n适应症：{}\n\n禁忌症：{}\n\n副作用：{}",
                    class.as_deref().unwrap_or("未知"),
                    indications.as_deref().unwrap_or("暂无"),
                    contras.as_deref().unwrap_or("暂无"),
                    side.as_deref().unwrap_or("暂无")
                );
                sqlx::query("INSERT OR IGNORE INTO flashcards (id, front, back, card_type, entity_id, difficulty) VALUES (?, ?, ?, 'drug', ?, 0.5)")
                    .bind(format!("fc_drug_{}", id))
                    .bind(&name)
                    .bind(&back)
                    .bind(&id)
                    .execute(&*pool).await.map_err(|e| e.to_string())?;
                count += 1;
            }
        }
        _ => return Err(format!("不支持的闪卡类型: {}", card_type)),
    }

    Ok(count)
}

/// 创建自定义闪卡
#[tauri::command]
pub async fn create_flashcard(
    pool: tauri::State<'_, DbPool>,
    front: String,
    back: String,
) -> Result<String, String> {
    let id = format!("fc_custom_{}", chrono::Utc::now().timestamp_millis());
    sqlx::query("INSERT INTO flashcards (id, front, back, card_type, difficulty) VALUES (?, ?, ?, 'custom', 0.5)")
        .bind(&id)
        .bind(&front)
        .bind(&back)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(id)
}

/// 删除闪卡
#[tauri::command]
pub async fn delete_flashcard(
    pool: tauri::State<'_, DbPool>,
    id: String,
) -> Result<(), String> {
    sqlx::query("DELETE FROM flashcards WHERE id = ?")
        .bind(&id)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 提交复习结果（SM-2 算法）
/// quality: 0-5（0=完全忘记，3=勉强想起，5=完美回忆）
#[tauri::command]
pub async fn review_flashcard(
    pool: tauri::State<'_, DbPool>,
    card_id: String,
    quality: i64,
) -> Result<(), String> {
    if !(0..=5).contains(&quality) {
        return Err("quality 必须在 0-5 之间".to_string());
    }

    // 获取上一次复习记录
    let last_review = sqlx::query(
        "SELECT interval_days, ease_factor FROM flashcard_reviews
         WHERE card_id = ? ORDER BY id DESC LIMIT 1"
    )
    .bind(&card_id)
    .fetch_optional(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    let (new_interval, new_ef, new_difficulty) = match last_review {
        Some(r) => {
            let old_interval: f64 = r.get("interval_days");
            let old_ef: f64 = r.get("ease_factor");
            // SM-2 算法
            let new_ef = (old_ef + (0.1 - (5.0 - quality as f64) * (0.08 + (5.0 - quality as f64) * 0.02))).max(1.3);
            let (interval, difficulty) = if quality < 3 {
                // 忘记了，重置
                (1.0, 0.8)
            } else if old_interval < 1.5 {
                (6.0, 0.5)
            } else {
                (old_interval * new_ef, 0.3)
            };
            (interval, new_ef, difficulty)
        }
        None => {
            // 首次复习
            let interval = if quality < 3 { 1.0 } else { if quality == 3 { 1.0 } else { 6.0 } };
            let ef = 2.5 + (0.1 - (5.0 - quality as f64) * (0.08 + (5.0 - quality as f64) * 0.02));
            (interval, ef.max(1.3), 0.5)
        }
    };

    let next_review = chrono::Utc::now() + chrono::Duration::milliseconds((new_interval * 86400000.0) as i64);
    let next_review_str = next_review.format("%Y-%m-%d %H:%M:%S").to_string();

    // 更新闪卡难度
    sqlx::query("UPDATE flashcards SET difficulty = ? WHERE id = ?")
        .bind(new_difficulty)
        .bind(&card_id)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    // 插入复习记录
    sqlx::query(
        "INSERT INTO flashcard_reviews (card_id, quality, interval_days, ease_factor, next_review)
         VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&card_id)
    .bind(quality)
    .bind(new_interval)
    .bind(new_ef)
    .bind(&next_review_str)
    .execute(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// 获取复习统计
#[tauri::command]
pub async fn get_review_stats(
    pool: tauri::State<'_, DbPool>,
) -> Result<ReviewStats, String> {
    let total_cards: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM flashcards")
        .fetch_one(&*pool).await.map_err(|e| e.to_string())?;

    let due_today: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT f.id) FROM flashcards f
         LEFT JOIN (
             SELECT card_id, MAX(id) AS max_id FROM flashcard_reviews GROUP BY card_id
         ) latest ON latest.card_id = f.id
         LEFT JOIN flashcard_reviews fr ON fr.id = latest.max_id
         WHERE fr.next_review IS NULL OR fr.next_review <= datetime('now')"
    ).fetch_one(&*pool).await.map_err(|e| e.to_string())?;

    let reviewed_today: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM flashcard_reviews WHERE date(reviewed_at) = date('now')"
    ).fetch_one(&*pool).await.map_err(|e| e.to_string())?;

    let mastered: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT f.id) FROM flashcards f
         INNER JOIN (
             SELECT card_id, MAX(id) AS max_id FROM flashcard_reviews GROUP BY card_id
         ) latest ON latest.card_id = f.id
         INNER JOIN flashcard_reviews fr ON fr.id = latest.max_id
         WHERE fr.ease_factor >= 2.3 AND fr.interval_days >= 21"
    ).fetch_one(&*pool).await.map_err(|e| e.to_string())?;

    Ok(ReviewStats {
        total_cards,
        due_today,
        reviewed_today,
        mastered,
    })
}
