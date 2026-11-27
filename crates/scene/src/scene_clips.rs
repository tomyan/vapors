//! A scene's clips (`display.rs` [`Scene`]): each item names its clip
//! by a `u32`, 0 for none or 1 + its place in the scene's list of the
//! distinct clips it uses, a boxed slice that grows by one when a clip
//! not yet in it is set, which is rare: a box paints under one clip, its
//! outline or thumb under another.

use crate::clip::Clip;
use crate::display::Scene;

impl Scene {
    /// Each item's clip, in the order of [`Scene::items`].
    pub fn clips(&self) -> impl ExactSizeIterator<Item = Option<Clip>> + '_ {
        self.clip_of.iter().map(|&name| self.clip_named(name))
    }

    /// Item `at`'s clip; `None` for an unclipped item or one past the end.
    #[must_use]
    pub fn clip_at(&self, at: usize) -> Option<Clip> {
        self.clip_of.get(at).and_then(|&name| self.clip_named(name))
    }

    /// Puts every item, and those pushed from now on, under `clip`.
    pub fn set_every_clip(&mut self, clip: Option<Clip>) {
        let name = self.name(clip);
        self.clip_of.fill(name);
        self.pen = name;
    }

    /// The clips the items name.
    pub(crate) fn palette(&self) -> &[Clip] {
        self.palette.as_deref().unwrap_or_default()
    }

    /// The clip `name` names.
    fn clip_named(&self, name: u32) -> Option<Clip> {
        let at = name.checked_sub(1)?;
        Some(self.palette()[at as usize])
    }

    /// The name of `clip`, added to the list when it is not in it.
    ///
    /// # Panics
    ///
    /// When a scene would name more than `u32::MAX` distinct clips.
    pub(crate) fn name(&mut self, clip: Option<Clip>) -> u32 {
        let Some(clip) = clip else { return 0 };
        let found = self.palette().iter().position(|each| *each == clip);
        let at = found.unwrap_or_else(|| self.add(clip));
        u32::try_from(at + 1).expect("a scene naming more than u32::MAX clips (scene_clips.rs)")
    }

    /// Adds `clip` to the list, returning its place.
    fn add(&mut self, clip: Clip) -> usize {
        let mut grown = self.palette.take().map(Vec::from).unwrap_or_default();
        grown.push(clip);
        self.palette = Some(grown.into_boxed_slice());
        self.palette().len() - 1
    }
}

/// Scenes are equal when their items are and each item's clip is, however
/// each list orders its clips.
impl PartialEq for Scene {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items && self.clips().eq(other.clips())
    }
}

#[cfg(test)]
#[path = "scene_clips_tests.rs"]
mod tests;
