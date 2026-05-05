use serde_json;

#[derive(serde::Serialize)]
pub struct ExportData {
    pub version: String,
    pub diseases: Vec<serde_json::Value>,
    pub symptoms: Vec<serde_json::Value>,
    pub drugs: Vec<serde_json::Value>,
    pub tests: Vec<serde_json::Value>,
    pub cases: Vec<serde_json::Value>,
}

#[tauri::command]
pub async fn export_data(
    _app: tauri::AppHandle,
    path: String,
) -> Result<String, String> {
    // TODO: 从数据库导出所有数据为 JSON
    let export = ExportData {
        version: "1.0".to_string(),
        diseases: vec![],
        symptoms: vec![],
        drugs: vec![],
        tests: vec![],
        cases: vec![],
    };

    let json = serde_json::to_string_pretty(&export).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(format!("数据已导出到: {}", path))
}

#[tauri::command]
pub async fn import_data(
    _app: tauri::AppHandle,
    path: String,
) -> Result<String, String> {
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let _data: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    // TODO: 验证并导入数据
    Ok(format!("数据已从 {} 导入", path))
}
