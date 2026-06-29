// 游戏命令层：8 个 Tauri 命令。
//
// 7 个转发到 virtual-vet sidecar（JSON-RPC over stdio）:
//   game_list_cases / game_new_session / game_advance / game_administer_drug
//   game_examine / game_diagnose / game_end_session
//
// 1 个本地执行（不进 sidecar）:
//   game_diagnosis_hint - 读症状映射表翻译后调 infer_diagnosis
//
// 设计原则:
//   - 症状层以 virtual-vet 为主: 前端展示 virtual-vet 的 display_name（原样），
//     仅在调用 infer_diagnosis 时翻译为 vet-knowledge 的 name_zh
//   - 映射表路径: {VET_VET_ROOT}/data/virtual_vet_to_vk_symptom_map.json
//   - 错误统一转 String（与 vet-knowledge 现有命令风格一致）

use crate::db::DbPool;
use crate::db::models::DiagnosisCandidate;
use crate::engine::{infer, DiagnosisInput};
use crate::sidecar::SidecarManager;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use std::env;
use std::path::PathBuf;

// ===== 数据类型（前端契约，对应 types/index.ts 的 Game* 接口）=====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameCaseSummary {
    pub id: String,
    pub title: String,
    pub difficulty: i64,
    pub difficulty_label: String,
    pub species: String,
    pub breed: String,
    pub age: String,
    pub weight_kg: f64,
    pub chief_complaint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSessionResponse {
    pub session_id: String,
    pub initial_snapshot: GameSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSnapshot {
    pub phase: String,
    pub medical_phase: String,
    pub time_elapsed_min: f64,
    pub time_budget_min: f64,
    pub time_remaining_min: f64,
    pub death_timer: Option<f64>,
    pub vitals: GameVitals,
    pub active_signs: Vec<GameActiveSign>,
    pub new_reports: Vec<Value>,
    pub pending_reports: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub case: Option<GameCaseInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    // 透传 process_action 的元信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_started_at_s: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_time_s: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_cost_min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameVitals {
    pub hr_bpm: f64,
    pub map_mmhg: f64,
    pub spo2_pct: f64,
    pub rr_bpm: f64,
    pub temp_c: f64,
    pub gfr_ml_min: f64,
    pub ph: f64,
    pub co_ml_min: f64,
    pub blood_volume_ml: f64,
    pub lactate_mmol_l: f64,
    pub bun_mg_dl: f64,
    pub game_time: String,
    pub is_night: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameActiveSign {
    pub sign_id: String,
    pub display_name: String,
    pub severity: String,
    pub organ_system: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clue_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub localizing_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameCaseInfo {
    pub id: String,
    pub title: String,
    pub difficulty: i64,
    pub difficulty_label: String,
    pub animal: GameAnimal,
    pub chief_complaint: String,
    pub history: String,
    pub time_budget_min: f64,
    pub starting_hints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameAnimal {
    pub species: String,
    pub breed: String,
    pub name: String,
    pub age: String,
    pub weight_kg: f64,
    pub sex: String,
}

// ===== Tauri 命令 =====

/// 列出所有可用病历（选项卡栏数据源）
#[tauri::command]
pub async fn game_list_cases(
    sidecar: tauri::State<'_, SidecarManager>,
) -> Result<Vec<GameCaseSummary>, String> {
    let result = sidecar
        .call("game.list_cases", serde_json::json!({}))
        .await
        .map_err(rpc_to_string)?;
    let cases_val = result.get("cases").ok_or("missing cases field")?;
    serde_json::from_value(cases_val.clone())
        .map_err(|e| format!("deserialize cases: {}", e))
}

/// 开新会话（开病历选项卡）
#[tauri::command]
pub async fn game_new_session(
    sidecar: tauri::State<'_, SidecarManager>,
    case_id: String,
) -> Result<NewSessionResponse, String> {
    let result = sidecar
        .call("game.new_session", serde_json::json!({"case_id": case_id}))
        .await
        .map_err(rpc_to_string)?;
    serde_json::from_value(result)
        .map_err(|e| format!("deserialize new_session: {}", e))
}

/// 推进时间（wait 10 分钟，消耗游戏内时间）
#[tauri::command]
pub async fn game_advance(
    sidecar: tauri::State<'_, SidecarManager>,
    session_id: String,
) -> Result<GameSnapshot, String> {
    let result = sidecar
        .call("game.advance", serde_json::json!({"session_id": session_id}))
        .await
        .map_err(rpc_to_string)?;
    serde_json::from_value(result).map_err(|e| format!("deserialize snapshot: {}", e))
}

/// 给药
#[tauri::command]
pub async fn game_administer_drug(
    sidecar: tauri::State<'_, SidecarManager>,
    session_id: String,
    drug_name: String,
    dose_mg_kg: Option<f64>,
    volume_ml: Option<f64>,
) -> Result<GameSnapshot, String> {
    let mut params = serde_json::json!({
        "session_id": session_id,
        "drug_name": drug_name,
    });
    if let Some(d) = dose_mg_kg {
        params["dose_mg_kg"] = serde_json::json!(d);
    }
    if let Some(v) = volume_ml {
        params["volume_ml"] = serde_json::json!(v);
    }
    let result = sidecar
        .call("game.administer_drug", params)
        .await
        .map_err(rpc_to_string)?;
    serde_json::from_value(result).map_err(|e| format!("deserialize snapshot: {}", e))
}

/// 开具检查
#[tauri::command]
pub async fn game_examine(
    sidecar: tauri::State<'_, SidecarManager>,
    session_id: String,
    test_type: String,
) -> Result<GameSnapshot, String> {
    let result = sidecar
        .call(
            "game.examine",
            serde_json::json!({"session_id": session_id, "test_type": test_type}),
        )
        .await
        .map_err(rpc_to_string)?;
    serde_json::from_value(result).map_err(|e| format!("deserialize snapshot: {}", e))
}

/// 提交诊断（终结会话，判定 win/lost）
#[tauri::command]
pub async fn game_diagnose(
    sidecar: tauri::State<'_, SidecarManager>,
    session_id: String,
    diagnosis: String,
) -> Result<GameSnapshot, String> {
    let result = sidecar
        .call(
            "game.diagnose",
            serde_json::json!({"session_id": session_id, "diagnosis": diagnosis}),
        )
        .await
        .map_err(rpc_to_string)?;
    serde_json::from_value(result).map_err(|e| format!("deserialize snapshot: {}", e))
}

/// 结束会话（关闭选项卡）
#[tauri::command]
pub async fn game_end_session(
    sidecar: tauri::State<'_, SidecarManager>,
    session_id: String,
) -> Result<(), String> {
    sidecar
        .call(
            "game.end_session",
            serde_json::json!({"session_id": session_id}),
        )
        .await
        .map_err(rpc_to_string)?;
    Ok(())
}

/// 本地诊断提示：将 virtual-vet display_name 翻译后调 infer_diagnosis。
///
/// 症状层以 virtual-vet 为主——前端展示用 display_name 原样，
/// 仅在此处读映射表翻译为 vet-knowledge 的 name_zh 再推理。
///
/// 映射表: {VET_VET_ROOT}/data/virtual_vet_to_vk_symptom_map.json
#[tauri::command]
pub async fn game_diagnosis_hint(
    pool: tauri::State<'_, DbPool>,
    symptoms: Vec<String>,
    species: String,
) -> Result<Vec<DiagnosisCandidate>, String> {
    // 1. 读映射表翻译症状
    let translated = translate_symptoms(&symptoms)?;

    // 2. 调用现有 infer_diagnosis 核心逻辑
    let input = DiagnosisInput {
        symptoms: translated,
        species: species.clone(),
        age: None,
        breed: None,
    };

    let disease_list: Vec<(String, String)> = sqlx::query(
        "SELECT DISTINCT d.id, d.name_zh
         FROM diseases d
         INNER JOIN disease_symptom ds ON ds.disease_id = d.id
         WHERE d.species LIKE ?",
    )
    .bind(format!("%{}%", species))
    .fetch_all(&*pool)
    .await
    .map_err(|e| e.to_string())?
    .iter()
    .map(|r| (r.get::<String, _>("id"), r.get::<String, _>("name_zh")))
    .collect();

    let mut all_disease_symptoms: std::collections::HashMap<String, Vec<(String, String, i64)>> =
        std::collections::HashMap::new();
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
            all_disease_symptoms
                .entry(did)
                .or_default()
                .push((sym_name, freq, is_patho));
        }
    }

    let mut diagnostics: std::collections::HashMap<String, Vec<(String, String)>> =
        std::collections::HashMap::new();
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

// ===== 辅助函数 =====

/// 将 RpcError 转为 String（vet-knowledge 命令层统一用 String 错误）
fn rpc_to_string(e: crate::sidecar::RpcError) -> String {
    format!("sidecar RPC error: {}", e)
}

/// 读 virtual_vet_to_vk_symptom_map.json 翻译症状。
///
/// 未匹配的症状静默跳过（vet-knowledge infer_diagnosis 本就容错）。
/// 映射表读取失败时返回原始症状（降级：直接喂 display_name，匹配率低但不崩溃）。
fn translate_symptoms(virtual_vet_symptoms: &[String]) -> Result<Vec<String>, String> {
    let project_root =
        env::var("VET_VET_ROOT").unwrap_or_else(|_| {
            r"C:\Users\ZhuanZ（无密码）\Desktop\Claudecode\01_代码实验\virtual-vet".to_string()
        });
    let map_path = PathBuf::from(&project_root)
        .join("data")
        .join("virtual_vet_to_vk_symptom_map.json");

    let map_text = match std::fs::read_to_string(&map_path) {
        Ok(t) => t,
        Err(e) => {
            // 映射表读取失败：降级为直接传原始症状
            eprintln!(
                "[game_diagnosis_hint] map file not found ({}), using raw symptoms",
                e
            );
            return Ok(virtual_vet_symptoms.to_vec());
        }
    };

    let map: Value = serde_json::from_str(&map_text)
        .map_err(|e| format!("parse symptom map: {}", e))?;
    let mappings = map
        .get("mappings")
        .ok_or("symptom map missing 'mappings' field")?;

    let mut translated: Vec<String> = Vec::new();
    for sym in virtual_vet_symptoms {
        match mappings.get(sym) {
            Some(v) if !v.is_null() => {
                if let Some(s) = v.as_str() {
                    translated.push(s.to_string());
                }
            }
            _ => {
                // null 或缺失：跳过（vet-knowledge 没有对应症状）
            }
        }
    }

    Ok(translated)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 真实 sidecar 返回的 snapshot（从 virtual-vet examine physical 抓取）
    // 用于对抗性验证 Rust 反序列化契约，避免再次发生 severity/localizing_value 类型不匹配
    const REAL_SNAPSHOT_EXAMINE: &str = r#"{
      "phase": "playing",
      "medical_phase": "worsening",
      "time_elapsed_min": 15,
      "time_budget_min": 120,
      "time_remaining_min": 105,
      "death_timer": null,
      "vitals": {
        "hr_bpm": 94.3, "map_mmhg": 110.0, "spo2_pct": 91.6, "rr_bpm": 18.9,
        "temp_c": 38.5, "gfr_ml_min": 81.0, "ph": 7.429, "co_ml_min": 1876.3,
        "blood_volume_ml": 1716.0, "lactate_mmol_l": 0.5, "bun_mg_dl": 27.6,
        "game_time": "08:15", "is_night": false
      },
      "active_signs": [
        {
          "sign_id": "dyspnea", "display_name": "呼吸困难",
          "severity": "mild", "organ_system": "respiratory",
          "clue_id": "dyspnea", "localizing_value": "organ_localizing"
        }
      ],
      "new_reports": [
        {
          "name": "血常规", "test_type": "blood_routine",
          "results": [
            {"param":"HCT","value":45.1,"unit":"%","normal_range":"37-55","flag":"normal"}
          ],
          "tags": ["dyspnea"], "summary": "血常规各项指标均在正常范围内。",
          "timestamp_s": 1020.0, "observed_at_s": 1020.0,
          "report_basis": "pre_advance", "available_after_min": 10, "available_at_s": 1620.0
        }
      ],
      "pending_reports": 0,
      "action_started_at_s": 720.0, "state_time_s": 1020.0,
      "time_cost_min": 5, "success": true
    }"#;

    // 真实 new_session 返回的 case summary
    const REAL_CASE_SUMMARY: &str = r#"{
      "id": "case_001", "title": "呕吐与嗜睡",
      "difficulty": 1, "difficulty_label": "★☆☆",
      "species": "犬", "breed": "金毛寻回犬",
      "age": "3岁", "weight_kg": 20.0,
      "chief_complaint": "咳嗽3天伴精神沉郁，昨夜呼吸急促、拒食"
    }"#;

    #[test]
    fn deserialize_real_snapshot_examine() {
        let v: Value = serde_json::from_str(REAL_SNAPSHOT_EXAMINE).unwrap();
        let snap: GameSnapshot = serde_json::from_value(v).unwrap();
        assert_eq!(snap.phase, "playing");
        assert_eq!(snap.medical_phase, "worsening");
        // severity 是字符串枚举（mild/moderate/severe/critical）
        assert_eq!(snap.active_signs[0].severity, "mild");
        // localizing_value 是字符串枚举（organ_localizing/highly_localizing/...）
        assert_eq!(snap.active_signs[0].localizing_value.as_deref(), Some("organ_localizing"));
        // new_reports 透传（Vec<Value>）
        assert_eq!(snap.new_reports.len(), 1);
        assert_eq!(snap.new_reports[0]["test_type"], "blood_routine");
    }

    #[test]
    fn deserialize_real_case_summary() {
        let v: Value = serde_json::from_str(REAL_CASE_SUMMARY).unwrap();
        let c: GameCaseSummary = serde_json::from_value(v).unwrap();
        assert_eq!(c.id, "case_001");
        assert_eq!(c.species, "犬");
        assert_eq!(c.weight_kg, 20.0);
    }
}
