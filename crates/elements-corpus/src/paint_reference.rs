//! Saved Chromium references for the paint tests
//! (`docs/plans/saved-chromium-references.md`): Chromium's screenshot of
//! each paint fixture, and of each changed state `gpu_paint_rules` holds
//! the rules to, at 1x and 2x, committed under `fixtures/references/`.
//! The paint tests compare vapors' rendering with these files and start
//! no browser. Nothing checks whether a file is still what Chromium would
//! draw: a mismatch is investigated as a vapors bug first, and a file is
//! captured again (`cargo xtask reference`, `paint_reference_capture`)
//! only when there is reason to suspect it, and looked at before it is
//! committed.

use std::path::{Path, PathBuf};

use vapors_text::comparison::{Rgba, read_png};

use crate::oracle::fixtures_dir;
pub use crate::paint_reference_capture::{capture, capture_into};
pub use crate::paint_reference_states::{STATES, State};
use crate::{Fixture, all};

/// The paint fixtures' window, in CSS pixels.
pub const WIDTH: u32 = 640;
/// See [`WIDTH`].
pub const HEIGHT: u32 = 400;
/// The scales every paint fixture and state is compared at.
pub const SCALES: [u32; 2] = [1, 2];

/// One saved reference: a paint fixture, as it is or in one of its
/// [`STATES`], at a scale.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reference {
    /// The paint fixture's name.
    pub fixture: String,
    /// The changed state, `None` for the fixture as it is.
    pub state: Option<&'static State>,
    /// Device pixels per CSS pixel.
    pub scale: u32,
}

impl Reference {
    /// The reference of `fixture` in `state` (`None` as it is) at `scale`.
    ///
    /// # Errors
    /// When `fixture` is not a paint fixture, `scale` is 0, or `state` is
    /// not one of `fixture`'s [`STATES`].
    pub fn of(fixture: &str, state: Option<&str>, scale: u32) -> Result<Self, String> {
        if scale == 0 {
            return Err("scale 0 draws nothing; fix: give 1 or 2".to_owned());
        }
        if !all().iter().any(|f| f.name == fixture && is_paint(f)) {
            return Err(format!(
                "{fixture} is not a paint fixture (no fixtures/{fixture}.wisp with `/* paint` \
                 in its style); fix: name one the paint tests compare"
            ));
        }
        let state = state.map(|name| State::of(fixture, name)).transpose()?;
        Ok(Self {
            fixture: fixture.to_owned(),
            state,
            scale,
        })
    }

    /// Its file under `fixtures/references/`, as `paint_boxes_2x.png` or
    /// `paint_boxes_card_radius_4_2x.png`.
    #[must_use]
    pub fn file_name(&self) -> String {
        match self.state {
            None => format!("{}_{}x.png", self.fixture, self.scale),
            Some(state) => format!("{}_{}_{}x.png", self.fixture, state.name, self.scale),
        }
    }

    /// The command that captures it with Chromium.
    #[must_use]
    pub fn command(&self) -> String {
        let state = self.state.map_or(String::new(), |s| format!(" {}", s.name));
        format!(
            "cargo xtask reference {} {}{state}",
            self.fixture, self.scale
        )
    }
}

/// Whether the paint tests compare `fixture`: a `/* paint` comment in
/// its style.
#[must_use]
pub fn is_paint(fixture: &Fixture) -> bool {
    fixture.style.contains("/* paint")
}

/// Every reference the paint tests read: each paint fixture and each
/// state at each of [`SCALES`].
#[must_use]
pub fn every_reference() -> Vec<Reference> {
    let fixtures = all().into_iter().filter(is_paint).map(|f| (f.name, None));
    let states = STATES.iter().map(|s| (s.fixture, Some(s)));
    fixtures
        .chain(states)
        .flat_map(|(fixture, state)| {
            SCALES.map(|scale| Reference {
                fixture: fixture.to_owned(),
                state,
                scale,
            })
        })
        .collect()
}

/// Where the saved references are committed.
#[must_use]
pub fn references_dir() -> PathBuf {
    fixtures_dir().join("references")
}

/// The committed reference of `fixture` in `state` at `scale`.
///
/// # Errors
/// As [`Reference::of`] and [`read`].
pub fn saved(fixture: &str, state: Option<&str>, scale: u32) -> Result<Rgba, String> {
    read(&references_dir(), &Reference::of(fixture, state, scale)?)
}

/// `reference`'s PNG in `dir`.
///
/// # Errors
/// When it is missing, unreadable, or not the window's size at its
/// scale; each names the file and the command that captures it.
pub fn read(dir: &Path, reference: &Reference) -> Result<Rgba, String> {
    let path = dir.join(reference.file_name());
    let refuse = |why: String| {
        format!(
            "{}: {why}; fix: if this reference is suspect, capture it with `{}`, \
             look at it, and commit it; else fix vapors",
            path.display(),
            reference.command()
        )
    };
    if !path.is_file() {
        return Err(refuse("no saved reference".to_owned()));
    }
    let image = read_png(&path).map_err(refuse)?;
    let size = [WIDTH * reference.scale, HEIGHT * reference.scale];
    if [image.width, image.height] != size {
        return Err(refuse(format!(
            "it is {}x{}, the window at {}x is {}x{}",
            image.width, image.height, reference.scale, size[0], size[1]
        )));
    }
    Ok(image)
}

#[cfg(test)]
#[path = "paint_reference_tests.rs"]
mod tests;
