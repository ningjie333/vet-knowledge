mod commands; mod db; mod engine;
use tauri::Manager;
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let ah = app.handle().clone();
            tauri::async_runtime::block_on(async move {
                let db = db::init(&ah).await.expect("DB init failed");
                ah.manage(db);
                // Search is handled via command handlers directly
                ah.manage(engine::InferenceEngine::new());
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::knowledge::get_diseases,
            commands::knowledge::get_disease_by_id,
            commands::knowledge::get_symptoms,
            commands::knowledge::get_symptom_by_id,
            commands::knowledge::get_diseases_by_symptom,
            commands::knowledge::get_drugs,
            commands::knowledge::get_drug_by_id,
            commands::knowledge::get_tests,
            commands::knowledge::get_test_by_id,
            commands::knowledge::get_cases,
            commands::knowledge::get_case_by_id,
            commands::knowledge::get_disease_ddx,
            commands::knowledge::get_disease_symptoms,
            commands::knowledge::get_disease_treatments,
            commands::knowledge::get_disease_diagnostics,
            commands::knowledge::get_disease_compare,
            commands::search::full_text_search,
            commands::diagnose::infer_diagnosis,
            commands::import_export::export_data,
            commands::import_export::import_data,
            // 闪卡系统
            commands::flashcards::get_due_flashcards,
            commands::flashcards::get_all_flashcards,
            commands::flashcards::generate_flashcards_from_knowledge,
            commands::flashcards::create_flashcard,
            commands::flashcards::delete_flashcard,
            commands::flashcards::review_flashcard,
            commands::flashcards::get_review_stats,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri failed");
}
