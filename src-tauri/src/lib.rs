mod admin;
mod commands;
mod crypto;
mod keychain;
mod s3_client;
mod secrets;
mod sync_engine;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::check_stored_key,
            commands::validate_key,
            commands::get_user_info,
            commands::logout,
            commands::start_upload,
            commands::start_download,
            commands::pause_sync,
            commands::resume_sync,
            commands::cancel_sync,
            commands::get_sync_progress,
            commands::list_cloud_folders,
            commands::delete_all_files,
            commands::check_credentials_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
