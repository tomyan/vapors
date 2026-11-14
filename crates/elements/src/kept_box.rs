//! One box's kept paint (IU3 3c-2, `docs/plans/iu3.md`): what its own
//! step, its outline and its thumbs put into the paint, its items
//! naming ids in the paint's slabs (`slab.rs`), and what it was made
//! from besides its box. A box kept from the last update
//! (`LayoutBox::kept`) whose values below are unchanged keeps its part
//! in place (`paint_partial.rs`); any other is painted again.

use vapors_scene::{Item, Scene};

pub(crate) use crate::kept_part::Part;
use crate::layout::{BoxRect, LayoutBox};
use crate::painted::Painted;

/// What a box's paint reads besides its box and style. Not here, as
/// they change only with a rebuilt box: border widths, which come only
/// from style (CSS has no % border); and whether its background became
/// the canvas's, which reads only html's and body's style, whose
/// restyle rebuilds the tree or body; and a text field's text origin,
/// which follows from its rect, padding, border and style, while a
/// value or style change rebuilds the field. The debug check
/// (`kept_paint_check.rs`) guards all three.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BoxFrom {
    /// Its border box, scroll offsets applied.
    pub(crate) rect: BoxRect,
    /// The clip of the boxes that cut it off.
    pub(crate) clip: Option<BoxRect>,
    /// Its padding widths, for its padding and content box (% padding
    /// follows the containing block's width).
    pub(crate) padding: taffy::Rect<f32>,
    /// The scroll frame its items, hits and entries are in (`frames.rs`).
    pub(crate) frame: usize,
}

/// One box's kept paint.
pub(crate) struct KeptBox {
    pub(crate) from: BoxFrom,
    /// Its own step's paint.
    pub(crate) own: Part,
    /// Its outline's and thumbs' items, when it has either: most boxes
    /// have neither, and two empty scenes are 144 bytes a slot.
    marks: Option<Box<Marks>>,
}

/// A box's outline and scroll thumbs.
#[derive(Default)]
struct Marks {
    outline: Scene,
    /// Painted again when they can have changed (`thumbs_again.rs`).
    thumb: Scene,
}

/// What a box with no outline or thumbs reads for them.
static NO_ITEMS: Scene = Scene::new();

impl BoxFrom {
    /// What box `index`'s paint reads now, under `clip` in scroll
    /// frame `frame`.
    pub(crate) fn of(
        (boxes, rects): (&[LayoutBox], &[BoxRect]),
        (clip, frame): (Option<BoxRect>, usize),
        index: usize,
    ) -> Self {
        Self {
            rect: rects[index],
            clip,
            padding: boxes[index].layout.padding,
            frame,
        }
    }
}

impl KeptBox {
    /// Its paint: its own part and its outline's items.
    pub(crate) fn new(from: BoxFrom, own: Part, outline: Scene) -> Self {
        let mut kept = Self {
            from,
            own,
            marks: None,
        };
        kept.set_marks(outline, Scene::new());
        kept
    }

    /// Whether its outline or thumbs, and its own part's rarer
    /// entries, are boxed apart.
    #[cfg(test)]
    pub(crate) fn boxed_apart(&self) -> (bool, bool) {
        (self.marks.is_some(), self.own.rare_boxed())
    }

    /// Its outline's items.
    pub(crate) fn outline(&self) -> &Scene {
        self.marks
            .as_ref()
            .map_or(&NO_ITEMS, |marks| &marks.outline)
    }

    /// Its scroll thumbs' items.
    pub(crate) fn thumb(&self) -> &Scene {
        self.marks.as_ref().map_or(&NO_ITEMS, |marks| &marks.thumb)
    }

    /// Its thumbs' items now `thumb`.
    pub(crate) fn set_thumb(&mut self, thumb: Scene) {
        let Some(marks) = self.marks.as_mut() else {
            return self.set_marks(Scene::new(), thumb);
        };
        marks.thumb = thumb;
        if marks.outline.items().is_empty() && marks.thumb.items().is_empty() {
            self.marks = None;
        }
    }

    /// Its outline's items, to move in place; none when it has none.
    pub(crate) fn outline_mut(&mut self) -> Option<&mut Scene> {
        self.marks.as_mut().map(|marks| &mut marks.outline)
    }

    /// `outline` and `thumb` kept, boxed apart only when either has
    /// items.
    fn set_marks(&mut self, outline: Scene, thumb: Scene) {
        let empty = outline.items().is_empty() && thumb.items().is_empty();
        self.marks = (!empty).then(|| Box::new(Marks { outline, thumb }));
    }

    /// The items `step`, one of this box's steps, paints: its own
    /// part's, its outline or its thumbs.
    pub(crate) fn scene_of(&self, step: crate::paint_order::Step) -> &Scene {
        match step {
            crate::paint_order::Step::Box(_) => &self.own.scene,
            crate::paint_order::Step::Thumb(_) => self.thumb(),
            crate::paint_order::Step::Outline(_) => self.outline(),
        }
    }

    /// How many scene items this box's own step and outline hold.
    pub(crate) fn items(&self) -> usize {
        self.own.scene.items().len() + self.outline().items().len()
    }

    /// Each scene item this box's paint holds: its own step's, its
    /// outline's, then its thumbs'.
    pub(crate) fn all_items(&self) -> impl Iterator<Item = &Item> {
        let outline = self.outline().items().iter();
        let own = self.own.scene.items().iter();
        own.chain(outline).chain(self.thumb().items())
    }

    /// Frees the slab ids this box's paint names and removes its
    /// entries from `painted`'s maps, as it is dropped.
    pub(crate) fn drop_from(&self, painted: &mut Painted) {
        painted.slabs.free(self.own.scene.items());
        painted.slabs.free(self.outline().items());
        self.own.leave(painted);
    }
}
