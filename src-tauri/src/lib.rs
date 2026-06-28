mod commands; mod db; mod engine;
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
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
                        // 用 dialog 弹窗显示错误，让用户能看到失败原因（替代 panic! 闪退）
                        // 依据 E-04 规范：panic! 只用于不可恢复的程序状态错误；
                        // 数据库初始化失败对用户而言是可恢复的（重新启动或修复文件）
                        let err_msg = format!(
                            "【启动失败】兽医知识库数据库初始化失败：\n\n{}\n\n错误日志已写入应用数据目录的 seed_import_error.log。\n请重启应用；若问题持续，请删除应用数据目录后重试。",
                            e
                        );
                        ah.dialog()
                            .message(err_msg)
                            .blocking_show();
                        std::process::exit(1);
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
