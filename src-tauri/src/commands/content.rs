use crate::AppData;
use percent_encoding::{percent_decode_str, utf8_percent_encode, AsciiSet, CONTROLS};
use rbook::Epub;
use std::path::Path;
use std::{error::Error, path::PathBuf};
use tauri::State;

const PATH_CHARS: &AsciiSet = &CONTROLS.add(b' ').add(b'#').add(b'%');

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

// To derive file paths, this should be useful: https://www.w3.org/TR/epub-33/#sec-container-iri
fn normalize_file_path(path: String, current_file_path: &str) -> String {
    // In case an EPUB passes a percent-encoded path
    let decoded = percent_decode_str(&path).decode_utf8_lossy();

    let current_file_path = Path::new(current_file_path);
    let path = Path::new(decoded.as_ref());

    let mut normalized_path = if !path.starts_with("/") {
        let current_dir = current_file_path.parent().unwrap_or_else(|| Path::new("/"));
        PathBuf::new().join(current_dir)
    } else {
        PathBuf::new()
    };

    for component in path {
        if component == ".." {
            normalized_path.pop();
        } else {
            normalized_path = normalized_path.join(component);
        }
    }

    let err_msg = "File paths MUST be encoded using UTF-8: https://www.w3.org/TR/epub-33/#sec-zip-container-zipreqs";
    utf8_percent_encode(normalized_path.to_str().expect(err_msg), PATH_CHARS).to_string()
}

pub(crate) fn get_resource(epub_source: &PathBuf, path: &str) -> Result<Resource, Box<dyn Error>> {
    let epub = Epub::open(epub_source).map_err(|e| e.to_string())?;

    let path = path.trim_start_matches('/');
    let resource = epub
        .manifest()
        .iter()
        .find(|entry| {
            let href = utf8_percent_encode(entry.href().as_str(), PATH_CHARS).to_string();
            href.contains(path)
        })
        .ok_or_else(|| format!("Resource not found at {path}"))?;

    let resource_bytes = resource.read_bytes()?;

    let content_type = resource.media_type();

    Ok(Resource::new(resource_bytes, content_type.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    mod normalize_file_path {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn test_basic_path() {
            // Common path you see for images
            let path = String::from("images/image.png");
            let current_file_path = "/OEBPS/cover.xhtml";
            let expected_path = "/OEBPS/images/image.png";
            assert_eq!(expected_path, normalize_file_path(path, current_file_path));
        }

        #[test]
        fn test_parent_traversal() {
            // Image is in a sibling directory
            let path = String::from("../images/image.png");
            let current_file_path = "/OEBPS/Text/chapter1.xhtml";
            let expected_path = "/OEBPS/images/image.png";
            assert_eq!(expected_path, normalize_file_path(path, current_file_path));
        }

        #[test]
        fn test_absolute_path() {
            // Absolute path from container root
            let path = String::from("/OEBPS/images/image.png");
            let current_file_path = "/OEBPS/Text/chapter1.xhtml";
            let expected_path = "/OEBPS/images/image.png";
            assert_eq!(expected_path, normalize_file_path(path, current_file_path));
        }

        #[test]
        fn test_same_directory() {
            // Image is in the same directory as the current file
            let path = String::from("image.png");
            let current_file_path = "/OEBPS/image.png";
            let expected_path = "/OEBPS/image.png";
            assert_eq!(expected_path, normalize_file_path(path, current_file_path));
        }

        #[test]
        fn test_deeply_nested() {
            let path = String::from("../../images/image.png");
            let current_file_path = "/OEBPS/Text/Section/chapter1.xhtml";
            let expected_path = "/OEBPS/images/image.png";
            assert_eq!(expected_path, normalize_file_path(path, current_file_path));
        }

        #[test]
        fn test_traversal_clamped_at_root() {
            // Spec requires .. past root stays at root: https://www.w3.org/TR/epub-33/#sec-container-iri
            let path = String::from("../../../../escape.png");
            let current_file_path = "/cover.xhtml";
            let expected_path = "/escape.png";
            assert_eq!(expected_path, normalize_file_path(path, current_file_path));
        }

        #[test]
        fn test_file_at_root() {
            // Current file is at the container root
            let path = String::from("images/image.png");
            let current_file_path = "/cover.xhtml";
            let expected_path = "/images/image.png";
            assert_eq!(expected_path, normalize_file_path(path, current_file_path));
        }

        #[test]
        fn test_already_percent_encoded_path() {
            let path = utf8_percent_encode("images/my image.png", PATH_CHARS).to_string();
            let current_file_path = "/OEBPS/cover.xhtml";
            let expected_path = "/OEBPS/images/my%20image.png";

            // Essentially testing that already-encoded
            assert_eq!(expected_path, normalize_file_path(path, current_file_path));
        }

        #[test]
        fn test_multiple_parent_then_descend() {
            // Go up then back down into a different subtree
            let path = String::from("../../fonts/arial.ttf");
            let current_file_path = "/OEBPS/Text/chapter1.xhtml";
            let expected_path = "/fonts/arial.ttf";
            assert_eq!(expected_path, normalize_file_path(path, current_file_path));
        }
    }
}
