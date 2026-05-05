use crate::commands::content::get_epub_content;
use crate::commands::metadata::read_epub_metadata;
use commands::content::get_resource;
use std::path::PathBuf;
use std::sync::Arc;
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
    let app_data = Arc::new(bootstrap_app());
    let protocol_data = app_data.clone(); // Create new copy to be used for epub uri scheme protocol

    tauri::Builder::default()
        .setup(move |app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            app.manage(app_data);

            Ok(())
        })
        .register_uri_scheme_protocol("epub", move |_ctx, request| {
            let path = request.uri().path();

            if let Ok(resource) = get_resource(&protocol_data.source, path) {
                let data = resource.bytes().to_owned();
                let content_type = resource.content_type();

                http::Response::builder()
                    .header(http::header::CONTENT_TYPE, content_type)
                    .body(data)
                    .unwrap()
            } else {
                http::Response::builder()
                    .status(http::StatusCode::BAD_REQUEST)
                    .header(http::header::CONTENT_TYPE, mime::TEXT_PLAIN.essence_str())
                    .body("failed to read file".as_bytes().to_vec())
                    .unwrap()
            }
        })
        .invoke_handler(tauri::generate_handler![
            read_epub_metadata,
            get_epub_content
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
