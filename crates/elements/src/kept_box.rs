//! One box's kept paint (IU3 3c-2, `docs/plans/iu3.md`): what its own
//! step and its outline put into the paint, with ids local to it, and
//! what it was made from besides its box. A box kept from the last
//! update (`LayoutBox::kept`) whose values below are unchanged has its
//! kept paint copied (`paint_partial.rs`); any other is painted again.

use std::rc::Rc;

use vapors_dom::NodeId;
use vapors_scene::{IconId, ImageId, Item, Scene, SurfaceId, TextId};

use crate::layout::{BoxRect, Content, LayoutBox, TextCache, TextKey};
use crate::painted::Painted;
use crate::style::Cursor;

/// What a box's paint reads besides its box and style.
#[derive(PartialEq)]
pub(crate) struct BoxFrom {
    /// Its border box, scroll offsets applied.
    pub(crate) rect: BoxRect,
    /// The clip of the boxes that cut it off.
    pub(crate) clip: Option<BoxRect>,
    /// Its border and padding widths, for its padding and content box.
    pub(crate) border: taffy::Rect<f32>,
    pub(crate) padding: taffy::Rect<f32>,
    /// Whether its background became the canvas's.
    pub(crate) canvas: bool,
    /// A text field's text origin: its text child's box.
    pub(crate) text_origin: Option<BoxRect>,
}

/// One box's kept paint.
pub(crate) struct KeptBox {
    pub(crate) from: BoxFrom,
    /// Its own step's paint.
    pub(crate) own: Part,
    /// Its outline's items.
    pub(crate) outline: Vec<Item>,
    /// The text cache's key for a text box's layout.
    pub(crate) text: Option<TextKey>,
}

/// What one step put into a paint; ids count from 0.
#[derive(Default)]
pub(crate) struct Part {
    scene: Scene,
    texts: Vec<Rc<vapors_text::Layout>>,
    images: Vec<Rc<vapors_scene::RgbaImage>>,
    icons: Vec<Rc<vapors_text::Icon>>,
    surfaces: Vec<String>,
    hits: Vec<(NodeId, BoxRect)>,
    cursors: Vec<(NodeId, Cursor)>,
    visible: Vec<NodeId>,
    rects: Vec<(NodeId, BoxRect)>,
    ranges: Vec<(NodeId, crate::range_paint::Track)>,
    text_origins: Vec<(NodeId, [f32; 2])>,
}

impl BoxFrom {
    /// What box `index`'s paint reads now; `canvas` is the element whose
    /// background became the canvas's.
    pub(crate) fn of(
        boxes: &[LayoutBox],
        rects: &[BoxRect],
        clip: Option<BoxRect>,
        index: usize,
        canvas: Option<NodeId>,
    ) -> Self {
        let layout_box = &boxes[index];
        Self {
            rect: rects[index],
            clip,
            border: layout_box.layout.border,
            padding: layout_box.layout.padding,
            canvas: canvas.is_some_and(|node| layout_box.element_in(index) == Some(node)),
            text_origin: layout_box
                .text_field
                .then(|| crate::text_paint::text_origin(boxes, rects, index)),
        }
    }
}

impl KeptBox {
    /// The text cache's key for box `index`'s layout, when it is a run.
    pub(crate) fn text_key(layout_box: &LayoutBox, rect: BoxRect, scale: f32) -> Option<TextKey> {
        match &layout_box.content {
            Content::Text(run) => Some(TextKey::of(run, Some(rect.width), scale)),
            Content::Element(_) | Content::Vacant => None,
        }
    }

    /// Keeps this box's text layout in `cache` for the next update.
    pub(crate) fn touch(&self, cache: &mut TextCache) {
        if let Some(key) = &self.text {
            cache.touch(key);
        }
    }

    /// How many scene items this box's paint holds.
    pub(crate) fn items(&self) -> usize {
        self.own.scene.items().len() + self.outline.len()
    }
}

impl Part {
    /// What `scratch` holds, leaving it empty.
    pub(crate) fn take(scratch: &mut Painted) -> Self {
        use std::mem::take;
        Self {
            scene: take(&mut scratch.scene),
            texts: take(&mut scratch.texts),
            images: take(&mut scratch.images),
            icons: take(&mut scratch.icons),
            surfaces: take(&mut scratch.surfaces),
            hits: take(&mut scratch.hits),
            cursors: scratch.cursors.drain().collect(),
            visible: scratch.visible.drain().collect(),
            rects: scratch.rects.drain().collect(),
            ranges: scratch.ranges.drain().collect(),
            text_origins: scratch.text_origins.drain().collect(),
        }
    }

    /// The items, each under its clip, then the rest, into `out`, every
    /// id moved past those already there.
    pub(crate) fn copy_into(&self, out: &mut Painted) {
        let ids = Offsets::of(out);
        for (item, clip) in self.scene.items().iter().zip(self.scene.clips()) {
            out.scene.set_clip(*clip);
            out.scene.push(ids.moved(*item));
        }
        out.texts.extend(self.texts.iter().cloned());
        out.images.extend(self.images.iter().cloned());
        out.icons.extend(self.icons.iter().cloned());
        out.surfaces.extend(self.surfaces.iter().cloned());
        out.hits.extend_from_slice(&self.hits);
        out.cursors.extend(self.cursors.iter().copied());
        out.visible.extend(self.visible.iter().copied());
        out.rects.extend(self.rects.iter().copied());
        out.ranges.extend(self.ranges.iter().copied());
        out.text_origins.extend(self.text_origins.iter().copied());
    }
}

/// How far a part's ids move when copied into a paint: past the texts,
/// images, icons and surfaces it already names.
struct Offsets([u32; 4]);

impl Offsets {
    fn of(out: &Painted) -> Self {
        let count = |len: usize| u32::try_from(len).expect("fewer than 2^32 of each");
        let lens = [out.texts.len(), out.images.len(), out.icons.len(), out.surfaces.len()];
        Self(lens.map(count))
    }

    fn moved(&self, item: Item) -> Item {
        let [text, image, icon, surface] = self.0;
        match item {
            Item::Text { text: id, origin, ink } => Item::Text { text: TextId(id.0 + text), origin, ink },
            Item::Image { image: id, rect } => Item::Image { image: ImageId(id.0 + image), rect },
            Item::Icon { icon: id, rect, colour } => Item::Icon { icon: IconId(id.0 + icon), rect, colour },
            Item::Surface { surface: id, rect } => {
                Item::Surface { surface: SurfaceId(id.0 + surface), rect }
            }
            Item::Shadow(_) | Item::Quad(_) => item,
        }
    }
}
