use crate::AppData;
use percent_encoding::{percent_decode_str, utf8_percent_encode, AsciiSet, CONTROLS};
use rbook::Epub;
use regex::Regex;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};
use tauri::State;

const PATH_CHARS: &AsciiSet = &CONTROLS.add(b' ').add(b'#').add(b'%');
static SRC_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"src="([^"]*)""#).unwrap());

/// Stores an EPUB resource's bytes and content-type, both of which are needed
/// to construct an HTTP response to serve the resource to the frontend.
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

    /// Return the raw bytes of the resource to be used in an HTTP response.
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Return the content-type of the resource to be used in an HTTP response.
    pub(crate) fn content_type(&self) -> &str {
        &self.content_type
    }
}

struct UrlInjector<'a> {
    current_file_path: &'a str,
}

impl<'a> UrlInjector<'a> {
    fn new(current_file_path: &'a str) -> Self {
        UrlInjector { current_file_path }
    }

    fn inject_resource_urls(&self, content: String) -> String {
        // E.g. an <img src="images/image.png"> tag inside /OEBPS/Text/chapter.xhtml will be transformed into <img src="epub://localhost/OEBPS/images/image.png">
        SRC_REGEX
            .replace_all(&content, |caps: &regex::Captures| {
                let file_path = String::from(&caps[1]);
                let final_src = if UrlInjector::is_local_resource(&file_path) {
                    format!("epub://localhost{}", self.normalize_file_path(file_path))
                } else {
                    file_path
                };

                format!(r#"src="{final_src}""#)
            })
            .into_owned()
    }

    fn normalize_file_path(&self, path: String) -> String {
        // In case an EPUB passes a percent-encoded path
        let decoded = percent_decode_str(&path).decode_utf8_lossy();

        let current_file_path = Path::new(self.current_file_path);
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

    fn is_local_resource(src: &str) -> bool {
        !(src.starts_with("http://")
            || src.starts_with("https://")
            || src.starts_with("data:")
            || src.starts_with("epub://")
            || src.starts_with('#'))
    }
}

#[tauri::command]
pub fn get_epub_content(state: State<'_, Arc<AppData>>) -> Result<String, String> {
    let source = &state.source;
    get_epub_content_inner(source).map_err(|e| e.to_string())
}

fn get_epub_content_inner(source: &PathBuf) -> anyhow::Result<String> {
    let epub = Epub::open(source)?;

    let mut content = String::new();

    // Loop through each entry in the manifest in canonical reading order
    // Each entry could be a chapter, image, etc.
    for spine_item in epub.spine().iter() {
        // Get the name of the current entry
        let id = &spine_item.idref();

        // Cross-reference the manifest to get the .xhtml file of the current
        // entry
        if let Some(resource) = epub.manifest().by_id(id) {
            let href = resource.href().as_str();
            let injector = UrlInjector::new(href);

            let cur_content = epub.read_resource_str(resource)?;

            content += &injector.inject_resource_urls(cur_content);
        }
    }

    Ok(content)
}

pub(crate) fn get_resource(epub_source: &PathBuf, path: &str) -> anyhow::Result<Resource> {
    let epub = Epub::open(epub_source)?;

    let resource = epub
        .manifest()
        .iter()
        .find(|entry| {
            let href = entry.href().as_str();
            href == path
        })
        .ok_or_else(|| anyhow::anyhow!("Resource not found at {path}"))?;

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

            let injector = UrlInjector::new(current_file_path);

            assert_eq!(expected_path, injector.normalize_file_path(path));
        }

        #[test]
        fn test_parent_traversal() {
            // Image is in a sibling directory
            let path = String::from("../images/image.png");
            let current_file_path = "/OEBPS/Text/chapter1.xhtml";
            let expected_path = "/OEBPS/images/image.png";

            let injector = UrlInjector::new(current_file_path);

            assert_eq!(expected_path, injector.normalize_file_path(path));
        }

        #[test]
        fn test_absolute_path() {
            // Absolute path from container root
            let path = String::from("/OEBPS/images/image.png");
            let current_file_path = "/OEBPS/Text/chapter1.xhtml";
            let expected_path = "/OEBPS/images/image.png";

            let injector = UrlInjector::new(current_file_path);

            assert_eq!(expected_path, injector.normalize_file_path(path));
        }

        #[test]
        fn test_same_directory() {
            // Image is in the same directory as the current file
            let path = String::from("image.png");
            let current_file_path = "/OEBPS/image.png";
            let expected_path = "/OEBPS/image.png";

            let injector = UrlInjector::new(current_file_path);

            assert_eq!(expected_path, injector.normalize_file_path(path));
        }

        #[test]
        fn test_deeply_nested() {
            let path = String::from("../../images/image.png");
            let current_file_path = "/OEBPS/Text/Section/chapter1.xhtml";
            let expected_path = "/OEBPS/images/image.png";

            let injector = UrlInjector::new(current_file_path);

            assert_eq!(expected_path, injector.normalize_file_path(path));
        }

        #[test]
        fn test_traversal_clamped_at_root() {
            // Spec requires .. past root stays at root: https://www.w3.org/TR/epub-33/#sec-container-iri
            let path = String::from("../../../../escape.png");
            let current_file_path = "/cover.xhtml";
            let expected_path = "/escape.png";

            let injector = UrlInjector::new(current_file_path);

            assert_eq!(expected_path, injector.normalize_file_path(path));
        }

        #[test]
        fn test_file_at_root() {
            // Current file is at the container root
            let path = String::from("images/image.png");
            let current_file_path = "/cover.xhtml";
            let expected_path = "/images/image.png";

            let injector = UrlInjector::new(current_file_path);

            assert_eq!(expected_path, injector.normalize_file_path(path));
        }

        #[test]
        fn test_already_percent_encoded_path() {
            let path = utf8_percent_encode("images/my image.png", PATH_CHARS).to_string();
            let current_file_path = "/OEBPS/cover.xhtml";
            let expected_path = "/OEBPS/images/my%20image.png";

            let injector = UrlInjector::new(current_file_path);

            // Essentially testing that already-encoded
            assert_eq!(expected_path, injector.normalize_file_path(path));
        }

        #[test]
        fn test_multiple_parent_then_descend() {
            // Go up then back down into a different subtree
            let path = String::from("../../fonts/arial.ttf");
            let current_file_path = "/OEBPS/Text/chapter1.xhtml";
            let expected_path = "/fonts/arial.ttf";

            let injector = UrlInjector::new(current_file_path);

            assert_eq!(expected_path, injector.normalize_file_path(path));
        }
    }

    #[test]
    fn test_inject_urls() {
        let current_file_path = "/OEBPS/cover.xhtml";

        let injector = UrlInjector::new(current_file_path);

        // Taken from No Longer Human:
        let original_xhtml = String::from(
            r#"
            <?xml version="1.0" encoding="UTF-8"?>
            <!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.1//EN"
              "http://www.w3.org/TR/xhtml11/DTD/xhtml11.dtd">

            <html xmlns="http://www.w3.org/1999/xhtml">
            <head>
          		<title>cover</title>
                <meta content="urn:uuid:c8e95494-7ba0-4c86-b3c0-b58f050b7d2f" name="Adept.expected.resource"/>
           	</head>
           	<body>
          		<div style="text-align:center;">
         			<img alt="cover.jpg" src="image/cover.jpg" style="max-width:100%;"/>
          		</div>
                <div style="float: none; margin: 10px 0px 10px 0px; text-align: center;"><p><a href="https://oceanofpdf.com"><i>OceanofPDF.com</i></a></p>
                </div>
            </body>
            </html>
        "#,
        );

        // Only difference here is that the img src has the custom URL
        let expected_xhtml = r#"
            <?xml version="1.0" encoding="UTF-8"?>
            <!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.1//EN"
              "http://www.w3.org/TR/xhtml11/DTD/xhtml11.dtd">

            <html xmlns="http://www.w3.org/1999/xhtml">
            <head>
          		<title>cover</title>
                <meta content="urn:uuid:c8e95494-7ba0-4c86-b3c0-b58f050b7d2f" name="Adept.expected.resource"/>
           	</head>
           	<body>
          		<div style="text-align:center;">
         			<img alt="cover.jpg" src="epub://localhost/OEBPS/image/cover.jpg" style="max-width:100%;"/>
          		</div>
                <div style="float: none; margin: 10px 0px 10px 0px; text-align: center;"><p><a href="https://oceanofpdf.com"><i>OceanofPDF.com</i></a></p>
                </div>
            </body>
            </html>
        "#;

        let modified_xhtml = injector.inject_resource_urls(original_xhtml);

        eprintln!("{modified_xhtml}");
        assert_eq!(modified_xhtml, expected_xhtml);
    }
}
