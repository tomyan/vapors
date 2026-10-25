//! Images registered with a document, by name, and what an `img`
//! element takes from them (F2c S10): its natural size and aspect ratio
//! in layout, and its pixels in paint. An `img` whose `src` names no
//! registered image is refused, naming it, with the rest of an `img`'s
//! refusals (`img_checks`); one without `src` is an empty box, as in
//! HTML. An image that cannot be drawn is refused when it is registered.

use std::collections::HashMap;
use std::rc::Rc;

use vapors_dom::NodeId;
use vapors_scene::{ImageId, Item, RgbaImage};

use crate::arena::Arena;
use crate::layout::{BoxRect, Content, LayoutBox};
use crate::paint::PaintInput;
use crate::painted::Painted;

/// The registered images.
#[derive(Debug, Default)]
pub(crate) struct Images {
    by_name: HashMap<String, Rc<RgbaImage>>,
}

impl Images {
    /// Registers `image` as `name`, replacing any image of that name.
    ///
    /// # Errors
    /// An image that cannot be drawn, naming it: a side of zero, a side
    /// over [`MAX_SIDE`], or pixels that do not match its size.
    pub(crate) fn register(&mut self, name: &str, image: RgbaImage) -> Result<(), String> {
        drawable(name, &image)?;
        self.by_name.insert(name.to_owned(), Rc::new(image));
        Ok(())
    }

    /// Whether an image is registered as `name`.
    pub(crate) fn has(&self, name: &str) -> bool {
        self.by_name.contains_key(name)
    }

    /// The image `node` shows: its `src`'s, when it is an `img` with one.
    pub(crate) fn of(&self, arena: &Arena, node: NodeId) -> Option<&Rc<RgbaImage>> {
        self.by_name.get(src(arena, node)?)
    }

    /// The natural size of what `node` shows, CSS px at one image pixel
    /// each, when it is an `img`: 0 x 0 without a `src`, as Chromium
    /// lays one out; `None` for any other element.
    #[allow(clippy::cast_precision_loss)] // image sizes are far below 2^24
    pub(crate) fn natural(&self, arena: &Arena, node: NodeId) -> Option<[f32; 2]> {
        let element = arena.get(node, "images").element()?;
        if element.tag != "img" {
            return None;
        }
        let size = self.of(arena, node).map(|image| image.size);
        Some(size.map_or([0.0, 0.0], |size| [size.width as f32, size.height as f32]))
    }
}

/// The longest side an image may have: WebGPU's guaranteed
/// `maxTextureDimension2D`, which every device vapors draws on supports.
pub(crate) const MAX_SIDE: u32 = 8192;

fn drawable(name: &str, image: &RgbaImage) -> Result<(), String> {
    let size = image.size;
    if size.width == 0 || size.height == 0 {
        return Err(format!(
            "the image {name:?} is {}x{}: a side of zero cannot be drawn; register an image with pixels",
            size.width, size.height
        ));
    }
    if size.width > MAX_SIDE || size.height > MAX_SIDE {
        return Err(format!(
            "the image {name:?} is {}x{}: a side over {MAX_SIDE} px cannot be uploaded on every device; scale it down first",
            size.width, size.height
        ));
    }
    let wanted = size.width as usize * size.height as usize * 4;
    if image.rgba.len() != wanted {
        return Err(format!(
            "the image {name:?} has {} bytes where {}x{} RGBA takes {wanted} bytes",
            image.rgba.len(),
            size.width,
            size.height
        ));
    }
    Ok(())
}

/// `node`'s `src`, when it is an `img`.
pub(crate) fn src(arena: &Arena, node: NodeId) -> Option<&str> {
    let element = arena.get(node, "images").element()?;
    if element.tag != "img" {
        return None;
    }
    element.attributes.get("src").map(String::as_str)
}

/// Paints the image `layout_box` shows over its content box (border box
/// `border_box`, CSS px), when it has one and is visible.
pub(crate) fn paint(
    layout_box: &LayoutBox,
    border_box: BoxRect,
    input: &PaintInput<'_>,
    out: &mut Painted,
) {
    let (Some(image), &Content::Element(node)) = (&layout_box.image, &layout_box.content) else {
        return;
    };
    if !input.used[&node].visible {
        return;
    }
    let content = content_box(layout_box, border_box);
    let id = ImageId(u32::try_from(out.images.len()).expect("fewer than 2^32 images"));
    out.images.push(Rc::clone(image));
    let rect = crate::paint::snapped(content, input.scale);
    out.scene.push(Item::Image { image: id, rect });
}

fn content_box(layout_box: &LayoutBox, border_box: BoxRect) -> BoxRect {
    let padding = crate::clip::padding_box(layout_box, border_box);
    let inner = layout_box.layout.padding;
    BoxRect {
        x: padding.x + inner.left,
        y: padding.y + inner.top,
        width: (padding.width - inner.left - inner.right).max(0.0),
        height: (padding.height - inner.top - inner.bottom).max(0.0),
    }
}
