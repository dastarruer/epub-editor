use crate::commands::content::get_epub_content;
use crate::commands::metadata::Metadata;
use crate::commands::metadata::read_epub_metadata;
use commands::content::get_resource;
use http::HeaderValue;
use rbook::Epub;
use rbook::ebook::errors::EbookError;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;

pub mod commands;

pub struct AppData {
    epub: EpubWrapper,
}

/// Stores data of the EPUB currently being edited.
pub(crate) struct EpubWrapper {
    epub: Epub,
    metadata: Metadata,
}

impl EpubWrapper {
    pub(crate) fn new(epub: Epub, metadata: Metadata) -> Self {
        Self { epub, metadata }
    }
}

/// # Panics
///
/// * If no source file is provided.
/// * If the path to the source file is invalid.
/// * If the EPUB at the provided path cannot be opened for some reason.
fn bootstrap_app() -> AppData {
    let source = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("No source file given, exiting..."),
    )
    .canonicalize()
    .unwrap_or_else(|e| {
        let err_msg = format!("Source file path is invalid: {e}");
        panic!("{}", err_msg);
    });

    let epub = match Epub::open(&source) {
        Ok(epub) => epub,
        Err(EbookError::Archive(e)) => {
            let err_msg = format!("Missing or invalid EPUB at {source:?}.\nError: {e}");
            panic!("{}", err_msg);
        }
        Err(EbookError::Format(e)) => {
            let err_msg = format!("Malformed EPUB at {source:?}.\nError: {e}");
            panic!("{}", err_msg);
        }
        Err(e) => {
            let err_msg = format!("Error opening EPUB at {source:?}.\nError: {e}");
            panic!("{}", err_msg);
        }
    };

    let metadata = Metadata::from(&epub);

    let epub = EpubWrapper::new(epub, metadata);

    AppData { epub }
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

            if let Ok(resource) = get_resource(&protocol_data.epub, path) {
                let data = resource.bytes().to_owned();
                let content_type = resource.content_type();

                let content_type_header =
                    HeaderValue::from_str(content_type).unwrap_or_else(|_| {
                        // 'application/octet-stream' is a stream of bytes
                        // Commonly used as a fallback when dealing with inavlid content types
                        HeaderValue::from_static(mime::APPLICATION_OCTET_STREAM.as_ref())
                    });

                http::Response::builder()
                    .header(http::header::CONTENT_TYPE, content_type_header)
                    .body(data)
                    .unwrap()
            } else {
                http::Response::builder()
                    .status(http::StatusCode::BAD_REQUEST)
                    .header(http::header::CONTENT_TYPE, mime::TEXT_PLAIN.essence_str())
                    .body(
                        "failed to read file"
                            .as_bytes()
                            .to_vec(),
                    )
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
