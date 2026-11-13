mod categories;
mod commands;
mod hasher;
mod metadata;
mod models;
mod mounts;
mod mover;
mod scanner;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::list_mounts,
            commands::system_info,
            commands::get_categories,
            commands::save_categories,
            commands::reset_categories,
            commands::pick_directory,
            commands::start_scan,
            commands::cancel_task,
            commands::get_summary,
            commands::find_duplicates,
            commands::get_duplicates,
            commands::file_meta,
            commands::plan_sort,
            commands::execute_sort,
            commands::get_log,
            commands::undo_last,
            commands::undo_batch,
            commands::reveal_path,
        ])
        .run(tauri::generate_context!())
        .expect("Fylr konnte nicht gestartet werden");
}
