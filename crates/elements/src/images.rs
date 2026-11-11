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
use crate::painting::Painting;

/// The registered images.
#[derive(Debug, Default)]
pub(crate) struct Images {
    by_name: HashMap<String, Rc<RgbaImage>>,
    /// The device's longest texture side, once the app has said it.
    max_side: Option<u32>,
}

impl Images {
    /// Registers `image` as `name`, replacing any image of that name.
    ///
    /// # Errors
    /// An image that cannot be drawn, naming it: a side of zero, a side
    /// over the device's limit (see [`side_problem`]), or pixels that do
    /// not match its size.
    pub(crate) fn register(&mut self, name: &str, image: RgbaImage) -> Result<(), String> {
        drawable(name, &image, self.max_side)?;
        self.by_name.insert(name.to_owned(), Rc::new(image));
        Ok(())
    }

    /// Drops the image registered as `name`, unless an `img` among
    /// `nodes` still names it.
    ///
    /// # Errors
    /// No image registered as `name`, or an `img` that still names it,
    /// naming the first.
    pub(crate) fn unregister(
        &mut self,
        name: &str,
        arena: &Arena,
        nodes: &[NodeId],
    ) -> Result<(), String> {
        if let Some(&node) = nodes.iter().find(|&&node| src(arena, node) == Some(name)) {
            let holder = crate::check::describe(arena, node);
            return Err(format!(
                "the image {name:?} is still shown by {holder}; remove the img or change its src first"
            ));
        }
        self.by_name
            .remove(name)
            .map(drop)
            .ok_or(format!("no image is registered as {name:?}"))
    }

    /// Check later images against `side`, the device's longest texture side.
    pub(crate) fn set_max_side(&mut self, side: u32) {
        self.max_side = Some(side);
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

/// WebGPU's guaranteed `maxTextureDimension2D`: every device vapors
/// draws on uploads a side this long.
pub(crate) const GUARANTEED_SIDE: u32 = 8192;

fn drawable(name: &str, image: &RgbaImage, max_side: Option<u32>) -> Result<(), String> {
    let RgbaImage { size, rgba } = image;
    let (width, height) = (size.width, size.height);
    let wanted = width as usize * height as usize * 4;
    let problem = if width == 0 || height == 0 {
        Some("a side of zero cannot be drawn; register an image with pixels".to_owned())
    } else if let Some(problem) = side_problem(width.max(height), max_side) {
        Some(problem)
    } else if rgba.len() != wanted {
        Some(format!(
            "it has {} bytes where RGBA at that size takes {wanted} bytes",
            rgba.len()
        ))
    } else {
        None
    };
    problem.map_or(Ok(()), |problem| {
        Err(format!("the image {name:?} is {width}x{height}: {problem}"))
    })
}

/// Why a longest side of `side` cannot be uploaded: over the device's
/// `max_side` when it is known; over [`GUARANTEED_SIDE`] when it is not,
/// since only that much is certain before the device is.
fn side_problem(side: u32, max_side: Option<u32>) -> Option<String> {
    match max_side {
        Some(limit) if side > limit => Some(format!(
            "a side over {limit} px is past this device's texture limit \
             (max_texture_dimension_2d {limit}); scale it down first"
        )),
        None if side > GUARANTEED_SIDE => Some(format!(
            "a side over {GUARANTEED_SIDE} px fits only some devices, and this document does not \
             know its device's limit yet (Document::set_max_image_side, which ElementsApp calls \
             every frame); register it after the first frame, or scale it to {GUARANTEED_SIDE} px"
        )),
        _ => None,
    }
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
    out: &mut Painting,
) {
    let (Some(image), &Content::Element(node)) = (&layout_box.image, &layout_box.content) else {
        return;
    };
    if !input.used[&node].visible {
        return;
    }
    let content = crate::clip::content_box(layout_box, border_box);
    let id = ImageId(out.slabs.images.insert(Rc::clone(image)));
    let rect = crate::paint::snapped(content, input.scale);
    out.scene.push(Item::Image { image: id, rect });
}
