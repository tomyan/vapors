//! The events fixtures played on vapors: their viewport, and reading
//! a fixture and its files.

use vapors_elements::Viewport;
use vapors_elements_corpus::oracle::fixtures_dir;
use vapors_elements_corpus::{Fixture, all};

/// The fixture's comment says 640x400 at 1x.
pub const VIEWPORT: Viewport = Viewport {
    width: 640.0,
    height: 400.0,
    scale: 1.0,
};

/// The fixture named `name`.
pub fn fixture(name: &str) -> Fixture {
    all()
        .into_iter()
        .find(|f| f.name == name)
        .expect("a fixture")
}

/// A file under `fixtures/`, as text.
pub fn read(file: &str) -> String {
    let path = fixtures_dir().join(file);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}
