//! What the file picker accepts, said once by the app and mapped to the
//! native dialog's filters and to the web input's `accept` attribute.

use crate::PlatformError;

/// One kind of file the picker offers: a description and its extensions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileKind {
    /// Shown by native dialogs, such as `PNG image`.
    pub description: String,
    /// Lower-case extensions without the dot, such as `png`.
    pub extensions: Vec<String>,
}

impl FileKind {
    /// A kind from its description and extensions. Extensions are
    /// lower-cased and a leading dot is dropped, so `.PNG` means `png`.
    pub fn new<'a>(description: &str, extensions: impl IntoIterator<Item = &'a str>) -> Self {
        Self {
            description: description.to_owned(),
            extensions: extensions
                .into_iter()
                .map(|e| e.trim_start_matches('.').to_ascii_lowercase())
                .collect(),
        }
    }
}

/// Refuse a list that would show the user an empty picker.
///
/// # Errors
///
/// [`PlatformError::Picker`] when `kinds` is empty or a kind has no extensions.
pub fn check(kinds: &[FileKind]) -> Result<(), PlatformError> {
    if kinds.is_empty() {
        return Err(PlatformError::Picker(
            "pick_file needs at least one FileKind; it was given none".into(),
        ));
    }
    if let Some(kind) = kinds.iter().find(|k| k.extensions.is_empty()) {
        return Err(PlatformError::Picker(format!(
            "FileKind \"{}\" has no extensions",
            kind.description
        )));
    }
    Ok(())
}

/// The web input's `accept`: `image/*` so phones offer their photo
/// library, then every extension as `.ext`.
#[cfg_attr(not(target_family = "wasm"), allow(dead_code))] // used on the web; tested here
pub fn accept(kinds: &[FileKind]) -> String {
    let mut accept = String::from("image/*");
    for extension in kinds.iter().flat_map(|k| &k.extensions) {
        accept.push_str(",.");
        accept.push_str(extension);
    }
    accept
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_and_jpeg() -> Vec<FileKind> {
        vec![
            FileKind::new("PNG image", ["png"]),
            FileKind::new("JPEG image", ["jpg", "jpeg"]),
        ]
    }

    #[test]
    fn extensions_are_lower_cased_without_their_dot() {
        // When
        let kind = FileKind::new("TIFF image", [".TIF", "tiff"]);

        // Then
        assert_eq!(kind.extensions, ["tif", "tiff"]);
    }

    #[test]
    fn accept_lists_images_then_every_extension() {
        // When
        let accept = accept(&png_and_jpeg());

        // Then
        assert_eq!(accept, "image/*,.png,.jpg,.jpeg");
    }

    #[test]
    fn an_empty_list_is_refused_by_name() {
        // When
        let error = check(&[]).expect_err("nothing to pick");

        // Then
        assert!(
            error.to_string().contains("at least one FileKind"),
            "{error}"
        );
    }

    #[test]
    fn a_kind_without_extensions_is_refused_by_description() {
        // Given
        let kinds = [FileKind::new("Anything", [])];

        // When
        let error = check(&kinds).expect_err("no extensions");

        // Then
        assert!(error.to_string().contains("\"Anything\""), "{error}");
    }

    #[test]
    fn a_full_list_passes() {
        // When / Then
        assert!(check(&png_and_jpeg()).is_ok());
    }
}
