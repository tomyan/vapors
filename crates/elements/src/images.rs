//! Images registered with a document, by name, and what an `img`
//! element takes from them (F2c S10): its natural size and aspect ratio
//! in layout, and its pixels in paint. An `img` whose `src` names no
//! registered image is refused, naming it; one without `src` is an empty
//! box, as in HTML. An inline `img` is refused: images inside a line of
//! text are inline-block content, which is not built.

use std::collections::HashMap;
use std::rc::Rc;

use vapors_dom::NodeId;
use vapors_scene::RgbaImage;

use crate::arena::Arena;
use crate::check::describe;
use crate::error::ElementError;
use crate::style::Used;

/// The registered images.
#[derive(Debug, Default)]
pub(crate) struct Images {
    by_name: HashMap<String, Rc<RgbaImage>>,
}

impl Images {
    /// Registers `image` as `name`, replacing any image of that name.
    pub(crate) fn register(&mut self, name: &str, image: RgbaImage) {
        self.by_name.insert(name.to_owned(), Rc::new(image));
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

    /// Every `img` in `nodes` with an unregistered `src`, or laid out
    /// inline.
    pub(crate) fn refusals(
        &self,
        arena: &Arena,
        nodes: &[NodeId],
        used: &HashMap<NodeId, Used>,
    ) -> Vec<ElementError> {
        let mut errors = Vec::new();
        for &node in nodes {
            let Some(name) = src(arena, node) else {
                continue;
            };
            if !self.by_name.contains_key(name) {
                errors.push(unregistered(arena, node, name));
            }
            if used.get(&node).is_some_and(|style| style.inline) {
                errors.push(inline(arena, node));
            }
        }
        errors
    }
}

/// `node`'s `src`, when it is an `img`.
fn src(arena: &Arena, node: NodeId) -> Option<&str> {
    let element = arena.get(node, "images").element()?;
    if element.tag != "img" {
        return None;
    }
    element.attributes.get("src").map(String::as_str)
}

fn unregistered(arena: &Arena, node: NodeId, name: &str) -> ElementError {
    ElementError::UnsupportedAttribute(format!(
        "{}: its src {name:?} names no image registered with the document; register it first with Document::register_image({name:?}, image)",
        describe(arena, node)
    ))
}

fn inline(arena: &Arena, node: NodeId) -> ElementError {
    ElementError::UnsupportedAttribute(format!(
        "{}: is inline, and an image inside a line of text is inline-block content, which vapors does not lay out yet (a slice after F2c); give it display: block, or put it in a flex or grid container",
        describe(arena, node)
    ))
}
