use rbook::Epub;
use std::sync::Arc;
use std::{error::Error, path::PathBuf};
use tauri::State;
use xml::reader::XmlEvent;
use xml::{EventReader, EventWriter};

use crate::AppData;

#[tauri::command]
pub fn get_epub_content(state: State<'_, Arc<AppData>>) -> Result<String, String> {
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

    content = inject_resource_urls(&content);

    Ok(content)
}

fn inject_resource_urls(content: &str) -> String {
    let mut reader = EventReader::from_str(content);

    let mut sink = Vec::new();
    let mut writer = EventWriter::new(&mut sink);

    while let Ok(event) = reader.next() {
        match &event {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                let mut updated_attributes = attributes.clone();

                if name.local_name == "img" {
                    if let Some(attribute) = updated_attributes
                        .iter_mut()
                        .find(|a| a.name.local_name == "src")
                    {
                        attribute.value = format!("epub://localhost/{}", attribute.value);
                    }
                }

                let mut event = xml::writer::events::XmlEvent::start_element(name.borrow());

                for attr in &updated_attributes {
                    event = event.attr(attr.name.borrow(), &attr.value);
                }

                let event = xml::writer::events::XmlEvent::from(event);
                writer
                    .write(event)
                    .expect("Writing img tag should not cause any issues. hopefully.");
            }
            XmlEvent::EndDocument => break,
            ref event => {
                if let Some(event) = event.as_writer_event() {
                    writer
                        .write(event)
                        .expect("Writing regular tags should not cause issues. i think.");
                }
            }
        };
    }

    String::from_utf8(sink).expect("EPUB should contain valid UTF-8.")
}

pub fn get_resource(epub_source: &PathBuf, path: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let epub = Epub::open(epub_source).map_err(|e| e.to_string())?;

    let resource = epub
        .manifest()
        .iter()
        .find(|entry| entry.href().as_str().contains(path))
        .ok_or_else(|| format!("Resource not found at {path}"))?;

    let resource_bytes = resource.read_bytes()?;

    Ok(resource_bytes)
}
