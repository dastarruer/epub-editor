use crate::commands::metadata::read_epub_metadata;
use crate::commands::spine::get_epub_content;
use std::path::PathBuf;
use tauri::Manager;

pub mod commands;

pub struct AppData {
    source: PathBuf,
}

fn bootstrap_app() -> AppData {
    let source = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("No source file given, exiting..."),
    );

    AppData { source }
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

            app.manage(bootstrap_app());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            read_epub_metadata,
            get_epub_content
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
