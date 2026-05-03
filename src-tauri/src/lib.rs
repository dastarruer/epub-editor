use crate::commands::metadata::read_epub_metadata;
use std::path::PathBuf;
use tauri::Manager;

pub mod commands;

pub struct AppData {
    source: PathBuf,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let source = PathBuf::from(
                std::env::args()
                    .nth(1)
                    .expect("No source file given, exiting..."),
            );

            app.manage(AppData { source });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![read_epub_metadata])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
