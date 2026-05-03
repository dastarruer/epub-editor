use crate::AppData;
use rbook::Epub;

use tauri::State;

#[derive(serde::Serialize)]
pub struct Metadata {
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

#[tauri::command]
pub async fn read_epub_metadata(state: State<'_, AppData>) -> Result<Metadata, String> {
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
