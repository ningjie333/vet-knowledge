use crate::db::DbPool;
use sqlx::Row;
use serde::Serialize;

/// SM-2 算法计算结果
#[derive(Debug, PartialEq)]
pub struct Sm2Result {
    pub interval_days: f64,
    pub ease_factor: f64,
    pub difficulty: f64,
}

/// SM-2 间隔重复算法（纯函数，无 DB 依赖）
///
/// 根据 SuperMemo SM-2 算法计算下次复习间隔与难度系数。
///
/// # 参数
/// * `quality` - 回忆质量 0-5（0=完全忘记，3=勉强想起，5=完美回忆）
/// * `old_interval` - 上次间隔天数（首次复习传 None）
/// * `old_ef` - 上次 EF 难度系数（首次复习传 None）
///
/// # 返回
/// `Sm2Result` 含新的 interval_days / ease_factor / difficulty
///
/// # 算法
/// - EF 更新：`EF' = max(1.3, EF + (0.1 - (5-q)*(0.08 + (5-q)*0.02)))`
/// - quality < 3：重置 interval=1，difficulty=0.8
/// - 非首次且 old_interval < 1.5：interval=6，difficulty=0.5
/// - 其他：interval = old_interval × new_ef，difficulty=0.3
pub fn calculate_sm2(
    quality: i64,
    old_interval: Option<f64>,
    old_ef: Option<f64>,
) -> Sm2Result {
    let q = quality as f64;

    match (old_interval, old_ef) {
        (Some(old_interval), Some(old_ef)) => {
            // 非首次复习：更新 EF
            let new_ef = (old_ef + (0.1 - (5.0 - q) * (0.08 + (5.0 - q) * 0.02))).max(1.3);
            let (interval, difficulty) = if quality < 3 {
                (1.0, 0.8)
            } else if old_interval < 1.5 {
                (6.0, 0.5)
            } else {
                (old_interval * new_ef, 0.3)
            };
            Sm2Result {
                interval_days: interval,
                ease_factor: new_ef,
                difficulty,
            }
        }
        _ => {
            // 首次复习（old_interval 或 old_ef 任一为 None）
            let interval = if quality < 3 {
                1.0
            } else if quality == 3 {
                1.0
            } else {
                6.0
            };
            let ef = (2.5 + (0.1 - (5.0 - q) * (0.08 + (5.0 - q) * 0.02))).max(1.3);
            Sm2Result {
                interval_days: interval,
                ease_factor: ef,
                difficulty: 0.5,
            }
        }
    }
}

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

    let (old_interval, old_ef) = match &last_review {
        Some(r) => {
            let interval: f64 = r.get("interval_days");
            let ef: f64 = r.get("ease_factor");
            (Some(interval), Some(ef))
        }
        None => (None, None),
    };

    let result = calculate_sm2(quality, old_interval, old_ef);
    let new_interval = result.interval_days;
    let new_ef = result.ease_factor;
    let new_difficulty = result.difficulty;

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
    let row = sqlx::query(
        "SELECT
          (SELECT COUNT(*) FROM flashcards) AS total_cards,
          (SELECT COUNT(DISTINCT f.id) FROM flashcards f
           LEFT JOIN (SELECT card_id, MAX(id) AS max_id FROM flashcard_reviews GROUP BY card_id) latest ON latest.card_id = f.id
           LEFT JOIN flashcard_reviews fr ON fr.id = latest.max_id
           WHERE fr.next_review IS NULL OR fr.next_review <= datetime('now')) AS due_today,
          (SELECT COUNT(*) FROM flashcard_reviews WHERE date(reviewed_at) = date('now')) AS reviewed_today,
          (SELECT COUNT(DISTINCT f.id) FROM flashcards f
           INNER JOIN (SELECT card_id, MAX(id) AS max_id FROM flashcard_reviews GROUP BY card_id) latest ON latest.card_id = f.id
           INNER JOIN flashcard_reviews fr ON fr.id = latest.max_id
           WHERE fr.ease_factor >= 2.3 AND fr.interval_days >= 21) AS mastered"
    )
    .fetch_one(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(ReviewStats {
        total_cards: row.get("total_cards"),
        due_today: row.get("due_today"),
        reviewed_today: row.get("reviewed_today"),
        mastered: row.get("mastered"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // === 首次复习 ===

    #[test]
    fn test_sm2_first_review_quality_0() {
        let result = calculate_sm2(0, None, None);
        assert!((result.interval_days - 1.0).abs() < 0.001);
        // q=0: ef = max(1.3, 2.5 + (0.1 - 5*(0.08+5*0.02))) = max(1.3, 2.5 - 0.8) = 1.7
        assert!((result.ease_factor - 1.7).abs() < 0.001);
        assert!((result.difficulty - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_sm2_first_review_quality_3() {
        let result = calculate_sm2(3, None, None);
        assert!((result.interval_days - 1.0).abs() < 0.001);
        // q=3: ef = max(1.3, 2.5 + (0.1 - 2*(0.08+2*0.02))) = max(1.3, 2.5 - 0.14) = 2.36
        assert!((result.ease_factor - 2.36).abs() < 0.001);
        assert!((result.difficulty - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_sm2_first_review_quality_5() {
        let result = calculate_sm2(5, None, None);
        assert!((result.interval_days - 6.0).abs() < 0.001);
        // q=5: ef = max(1.3, 2.5 + 0.1) = 2.6
        assert!((result.ease_factor - 2.6).abs() < 0.001);
        assert!((result.difficulty - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_sm2_first_review_quality_4() {
        let result = calculate_sm2(4, None, None);
        // q=4: interval=6 (quality >= 4)
        assert!((result.interval_days - 6.0).abs() < 0.001);
        // q=4: ef = max(1.3, 2.5 + 0) = 2.5
        assert!((result.ease_factor - 2.5).abs() < 0.001);
    }

    // === 非首次复习：quality < 3 重置 ===

    #[test]
    fn test_sm2_repeat_quality_0_reset() {
        let result = calculate_sm2(0, Some(10.0), Some(2.5));
        assert!((result.interval_days - 1.0).abs() < 0.001);
        assert!((result.difficulty - 0.8).abs() < 0.001);
        // ef 仍会更新：max(1.3, 2.5 - 0.8) = 1.7
        assert!((result.ease_factor - 1.7).abs() < 0.001);
    }

    #[test]
    fn test_sm2_repeat_quality_2_reset() {
        let result = calculate_sm2(2, Some(10.0), Some(2.5));
        assert!((result.interval_days - 1.0).abs() < 0.001);
        assert!((result.difficulty - 0.8).abs() < 0.001);
        // q=2: ef = max(1.3, 2.5 + (0.1 - 3*(0.08+3*0.02))) = max(1.3, 2.5 - 0.32) = 2.18
        assert!((result.ease_factor - 2.18).abs() < 0.001);
    }

    // === 非首次复习：quality >= 3 且 old_interval < 1.5 ===

    #[test]
    fn test_sm2_repeat_quality_3_short_interval() {
        let result = calculate_sm2(3, Some(1.0), Some(2.5));
        assert!((result.interval_days - 6.0).abs() < 0.001);
        assert!((result.difficulty - 0.5).abs() < 0.001);
        assert!((result.ease_factor - 2.36).abs() < 0.001);
    }

    // === 非首次复习：quality >= 3 且 old_interval >= 1.5 ===

    #[test]
    fn test_sm2_repeat_quality_5_normal_interval() {
        let result = calculate_sm2(5, Some(2.0), Some(2.5));
        // q=5: ef = 2.6, interval = 2.0 * 2.6 = 5.2
        assert!((result.interval_days - 5.2).abs() < 0.001);
        assert!((result.difficulty - 0.3).abs() < 0.001);
        assert!((result.ease_factor - 2.6).abs() < 0.001);
    }

    #[test]
    fn test_sm2_repeat_quality_3_normal_interval() {
        let result = calculate_sm2(3, Some(2.0), Some(2.5));
        // q=3: ef = 2.36, interval = 2.0 * 2.36 = 4.72
        assert!((result.interval_days - 4.72).abs() < 0.001);
        assert!((result.difficulty - 0.3).abs() < 0.001);
        assert!((result.ease_factor - 2.36).abs() < 0.001);
    }

    // === EF 下限保护 ===

    #[test]
    fn test_sm2_ef_floor_1_3() {
        // old_ef=1.3, q=0 → ef = max(1.3, 1.3 - 0.8) = max(1.3, 0.5) = 1.3
        let result = calculate_sm2(0, Some(10.0), Some(1.3));
        assert!((result.ease_factor - 1.3).abs() < 0.001, "EF 不应低于 1.3");
    }

    #[test]
    fn test_sm2_ef_floor_negative_ef() {
        // old_ef=1.0（低于下限，模拟异常数据），q=0 → ef = max(1.3, 1.0 - 0.8) = 1.3
        let result = calculate_sm2(0, Some(10.0), Some(1.0));
        assert!((result.ease_factor - 1.3).abs() < 0.001, "异常低 EF 应被提升到 1.3");
    }

    // === EF 计算验证 ===

    #[test]
    fn test_sm2_ef_increase_quality_5() {
        let result = calculate_sm2(5, Some(2.0), Some(2.5));
        // q=5: EF 应增加 0.1
        assert!((result.ease_factor - 2.6).abs() < 0.001);
    }

    #[test]
    fn test_sm2_ef_unchanged_quality_4() {
        let result = calculate_sm2(4, Some(2.0), Some(2.5));
        // q=4: EF 不变
        assert!((result.ease_factor - 2.5).abs() < 0.001);
    }

    #[test]
    fn test_sm2_ef_decrease_quality_3() {
        let result = calculate_sm2(3, Some(2.0), Some(2.5));
        // q=3: EF 减少 0.14
        assert!((result.ease_factor - 2.36).abs() < 0.001);
    }

    // === 边界值 ===

    #[test]
    fn test_sm2_boundary_quality_2() {
        // q=2 < 3，应触发重置
        let result = calculate_sm2(2, Some(5.0), Some(2.5));
        assert!((result.interval_days - 1.0).abs() < 0.001);
        assert!((result.difficulty - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_sm2_boundary_quality_3() {
        // q=3 >= 3，不应触发重置
        let result = calculate_sm2(3, Some(2.0), Some(2.5));
        assert!(result.interval_days > 1.0, "q=3 不应重置为 1 天");
        assert!((result.difficulty - 0.3).abs() < 0.001);
    }

    #[test]
    fn test_sm2_boundary_interval_1_5() {
        // old_interval=1.5 不满足 < 1.5，走第三分支
        let result = calculate_sm2(5, Some(1.5), Some(2.5));
        // interval = 1.5 * 2.6 = 3.9
        assert!((result.interval_days - 3.9).abs() < 0.001);
        assert!((result.difficulty - 0.3).abs() < 0.001);
    }

    #[test]
    fn test_sm2_boundary_interval_just_below_1_5() {
        // old_interval=1.499 < 1.5，走第二分支
        let result = calculate_sm2(5, Some(1.499), Some(2.5));
        assert!((result.interval_days - 6.0).abs() < 0.001);
        assert!((result.difficulty - 0.5).abs() < 0.001);
    }

    // === 完整复习链路模拟 ===

    #[test]
    fn test_sm2_review_chain_mastered() {
        // 模拟连续高质量复习的间隔增长
        let r1 = calculate_sm2(5, None, None);
        assert!((r1.interval_days - 6.0).abs() < 0.001);
        assert!((r1.ease_factor - 2.6).abs() < 0.001);

        let r2 = calculate_sm2(5, Some(r1.interval_days), Some(r1.ease_factor));
        // old_interval=6.0 >= 1.5, ef=2.6 → interval = 6.0 * 2.7 = 16.2
        // q=5: ef = 2.6 + 0.1 = 2.7
        assert!((r2.ease_factor - 2.7).abs() < 0.001);
        assert!((r2.interval_days - 16.2).abs() < 0.001);

        let r3 = calculate_sm2(5, Some(r2.interval_days), Some(r2.ease_factor));
        // old_interval=16.2, ef=2.8 → interval = 16.2 * 2.8 = 45.36
        assert!((r3.ease_factor - 2.8).abs() < 0.001);
        assert!(r3.interval_days > 21.0, "连续高质量复习后间隔应超过 21 天");
    }

    #[test]
    fn test_sm2_review_chain_forgot_then_recover() {
        // 模拟忘记后重新掌握
        let r1 = calculate_sm2(5, Some(10.0), Some(2.5));
        assert!((r1.interval_days - 26.0).abs() < 0.001); // 10 * 2.6

        let r2 = calculate_sm2(0, Some(r1.interval_days), Some(r1.ease_factor));
        assert!((r2.interval_days - 1.0).abs() < 0.001); // 重置
        assert!((r2.difficulty - 0.8).abs() < 0.001);

        let r3 = calculate_sm2(5, Some(r2.interval_days), Some(r2.ease_factor));
        // 重新掌握，但 interval=1.0 < 1.5，走第二分支：interval=6, difficulty=0.5
        assert!((r3.interval_days - 6.0).abs() < 0.001);
        assert!((r3.difficulty - 0.5).abs() < 0.001);
    }
}
