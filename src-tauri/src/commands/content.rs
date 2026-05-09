use rbook::Epub;
use tauri::State;

use crate::AppData;

pub(crate) struct Resource {
    bytes: Vec<u8>,
    content_type: String,
}

impl Resource {
    pub(crate) fn new(bytes: Vec<u8>, content_type: String) -> Self {
        Self {
            bytes,
            content_type,
        }
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(crate) fn content_type(&self) -> &str {
        &self.content_type
    }
}

#[tauri::command]
pub fn get_epub_content(state: State<'_, AppData>) -> Result<String, String> {
    let source = &state.source;

    let epub = Epub::open(source).map_err(|e| e.to_string())?;

    let mut content = String::new();

    // Loop through each entry in the manifest in canonical reading order
    // Each entry could be a chapter, image, etc.
    for spine_item in epub.spine().iter() {
        // Get the name of the current entry
        let id = &spine_item.idref();

        // Cross-reference the manifest to get the .xhtml file of the current
        // entry
        if let Some(resource) = epub.manifest().by_id(id) {
            let cur_content = epub
                .read_resource_str(resource)
                .map_err(|e| e.to_string())?;

            content += &cur_content;
        }
    }

    Ok(content)
}
