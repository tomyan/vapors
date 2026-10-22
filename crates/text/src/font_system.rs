//! The fonts a process can shape with: the bundle registered first, then
//! the system's fonts as fallback where there is a system to ask.

use std::sync::Arc;

use parley::fontique::{Blob, Collection, CollectionOptions, FallbackKey, GenericFamily, Script};
use parley::{FontContext, FontData, LayoutContext, RangedBuilder};
use vapors_scene::Color;

use crate::fonts::{GEIST, GEIST_MONO};

/// Font collection plus Parley's layout scratch space. One per app.
pub struct FontSystem {
    fonts: FontContext,
    layouts: LayoutContext<Color>,
    bundled: Vec<u64>,
    system_fonts: bool,
}

impl FontSystem {
    /// The bundle, then system fonts as fallback. On the web there are
    /// no system fonts and this is [`FontSystem::bundle_only`].
    #[must_use]
    pub fn new() -> Self {
        Self::with_system_fonts(cfg!(not(target_family = "wasm")))
    }

    /// The bundle alone: what the web gets, and what tests use to show
    /// the web's behaviour on native.
    #[must_use]
    pub fn bundle_only() -> Self {
        Self::with_system_fonts(false)
    }

    fn with_system_fonts(system_fonts: bool) -> Self {
        let mut collection = Collection::new(CollectionOptions {
            shared: false,
            system_fonts,
        });
        let bundled = register_bundle(&mut collection);
        Self {
            fonts: FontContext {
                collection,
                source_cache: parley::fontique::SourceCache::default(),
            },
            layouts: LayoutContext::new(),
            bundled,
            system_fonts,
        }
    }

    /// True when system fonts were asked for and at least one family
    /// beyond the bundle is known.
    #[must_use]
    pub fn has_system_fonts(&mut self) -> bool {
        self.system_fonts && self.fonts.collection.family_names().count() > 2
    }

    /// Register the fonts in `bytes` and make their families the fallback
    /// for `script` (an ISO 15924 tag such as `*b"Hani"`), after any
    /// fallback already there. This is how the web will add a fetched
    /// font for a script the bundle lacks. Returns how many families the
    /// data held; zero means it was not a font.
    pub fn register_fallback(&mut self, bytes: Vec<u8>, script: [u8; 4]) -> usize {
        let families = self
            .fonts
            .collection
            .register_fonts(Blob::from(bytes), None);
        let ids: Vec<_> = families.iter().map(|(id, _)| *id).collect();
        let key = FallbackKey::new(Script::from_bytes(script), None);
        self.fonts
            .collection
            .append_fallbacks(key, ids.iter().copied());
        ids.len()
    }

    /// True when `font` is one of the bundled files.
    #[must_use]
    pub fn is_bundled(&self, font: &FontData) -> bool {
        self.bundled.contains(&font.data.id())
    }

    pub(crate) fn ranged_builder<'a>(
        &'a mut self,
        text: &'a str,
        scale: f32,
    ) -> RangedBuilder<'a, Color> {
        self.layouts
            .ranged_builder(&mut self.fonts, text, scale, false)
    }
}

impl Default for FontSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Register both bundled fonts and make them the generic sans-serif and
/// monospace, so a CSS-style family list ends in the bundle.
fn register_bundle(collection: &mut Collection) -> Vec<u64> {
    let mut ids = Vec::new();
    for (bytes, generic) in [
        (GEIST, GenericFamily::SansSerif),
        (GEIST_MONO, GenericFamily::Monospace),
    ] {
        let blob = Blob::new(Arc::new(bytes));
        ids.push(blob.id());
        let families = collection.register_fonts(blob, None);
        let family_ids = families.iter().map(|(id, _)| *id).collect::<Vec<_>>();
        collection.set_generic_families(generic, family_ids.into_iter());
    }
    ids
}
