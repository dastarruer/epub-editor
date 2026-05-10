use rbook::Epub;
use std::sync::Arc;
use tauri::State;

use crate::AppData;

/// Stores metadata of an EPUB.
#[derive(serde::Serialize, Clone)]
pub struct Metadata {
    title: Option<String>,
    year: Option<i16>,
    creators: Vec<String>,
}

impl From<&Epub> for Metadata {
    fn from(value: &Epub) -> Self {
        let title = value
            .metadata()
            .title()
            .map(|t| t.value().to_string());
        let year = value
            .metadata()
            .published()
            .map(|y| y.date().year());
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

#[tauri::command]
pub fn read_epub_metadata(state: State<'_, Arc<AppData>>) -> Metadata {
    state.epub.metadata.to_owned()
}
