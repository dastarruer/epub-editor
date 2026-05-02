use rbook::Epub;
use std::path::PathBuf;
use tauri::Manager;
use tauri::State;

struct AppData {
    source: PathBuf,
}

#[derive(serde::Serialize)]
struct Metadata {
    title: Option<String>,
    year: Option<i16>,
    creators: Vec<String>,
}

impl From<Epub> for Metadata {
    fn from(value: Epub) -> Self {
        let title = value.metadata().title().map(|t| t.value().to_string());
        let year = value.metadata().published().map(|y| y.date().year());
        let creators = value
            .metadata()
            .creators()
            .map(|c| c.value().to_string())
            .collect();

        Self {
            title,
            year,
            creators,
        }
    }
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

#[tauri::command]
async fn read_epub_metadata(state: State<'_, AppData>) -> Result<Metadata, String> {
    let source = &state.source;

    // Skip manifest and spine, since we just want metadata right now
    let epub = Epub::options()
        .skip_toc(true)
        .skip_manifest(true)
        .skip_spine(true)
        .open(source)
        .map_err(|e| e.to_string())?;

    Ok(epub.into())
}
