mod commands;
mod downloader;
mod settings;
mod updater;
mod utils;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(downloader::new_process_handle())
        .invoke_handler(tauri::generate_handler![
            commands::check_dependencies,
            commands::get_settings,
            commands::save_settings,
            commands::start_download,
            commands::cancel_download,
            commands::check_ytdlp_update,
            commands::update_ytdlp,
            commands::check_ffmpeg,
            commands::install_ffmpeg,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
