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
                match db::init(&ah).await {
                    Ok(db) => {
                        ah.manage(db);
                    }
                    Err(e) => {
                        db::write_import_error_log(&ah, &e);
                        eprintln!("[vet-knowledge] DB init failed: {}", e);
                        panic!("DB init failed: {}", e);
                    }
                }
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
            commands::knowledge::get_case_diseases,
            commands::knowledge::get_disease_ddx,
            commands::knowledge::get_disease_symptoms,
            commands::knowledge::get_disease_treatments,
            commands::knowledge::get_disease_diagnostics,
            commands::knowledge::get_disease_compare,
            // 治疗 & 标签
            commands::treatments::get_treatments,
            commands::treatments::get_treatment_by_id,
            commands::treatments::get_disease_treatment_map,
            commands::treatments::get_tags,
            commands::treatments::get_entity_tags,
            commands::treatments::get_entities_by_tag,
            commands::treatments::add_entity_tag,
            commands::treatments::remove_entity_tag,
            commands::search::full_text_search,
            commands::diagnose::infer_diagnosis,
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
