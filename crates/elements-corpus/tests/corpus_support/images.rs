//! The fixtures' images registered with a document, by file name, as
//! the oracle page finds them beside itself.

use vapors_canvas::decode_png;
use vapors_elements::Document;
use vapors_elements_corpus::oracle::image_files;

/// Registers every fixture image with `doc`.
///
/// # Panics
/// When an image cannot be read or decoded, naming it.
pub fn register_fixture_images(doc: &mut Document) {
    for (name, bytes) in image_files().expect("the fixture images") {
        let image = decode_png(&bytes).unwrap_or_else(|e| panic!("fixtures/images/{name}: {e}"));
        doc.register_image(&name, image)
            .unwrap_or_else(|e| panic!("fixtures/images/{name}: {e}"));
    }
}
