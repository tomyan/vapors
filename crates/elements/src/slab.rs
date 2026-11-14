//! The texts, images, icons and surfaces a paint's scene items name, by
//! id (IU3 3c-3d1, `docs/plans/iu3.md`). Each is held in a slab: a
//! box's part names ids that stay while the part is kept, and a part
//! painted again or dropped frees its ids for later parts to take. Ids
//! are neither dense nor in paint order.

use std::rc::Rc;

use vapors_scene::{IconId, ImageId, Item, RgbaImage, SurfaceId, TextId};
use vapors_text::{Icon, Layout};

/// Values by id: a vacant id is on the free list, to be taken next.
pub(crate) struct Slab<T> {
    entries: Vec<Option<T>>,
    free: Vec<u32>,
}

impl<T> Default for Slab<T> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            free: Vec::new(),
        }
    }
}

impl<T> Slab<T> {
    /// Holds `value` at a free id, or a new one; returns the id.
    pub(crate) fn insert(&mut self, value: T) -> u32 {
        if let Some(id) = self.free.pop() {
            self.entries[id as usize] = Some(value);
            return id;
        }
        self.entries.push(Some(value));
        u32::try_from(self.entries.len() - 1).expect("fewer than 2^32 ids")
    }

    /// Frees `id` of the `kind` slab. Panics, naming the kind and id, when
    /// it is not held: a part freed twice, or an id no part took.
    pub(crate) fn remove(&mut self, kind: &str, id: u32) {
        let entry = self.entries.get_mut(id as usize).and_then(Option::take);
        assert!(
            entry.is_some(),
            "{kind} id {id} is free or was never issued, yet a part frees it \
             (paint_partial.rs, slab.rs); reproduce with `cargo xtask test`"
        );
        self.free.push(id);
    }

    /// The value at `id`; `None` when it is free or was never taken.
    pub(crate) fn get(&self, id: u32) -> Option<&T> {
        self.entries.get(id as usize)?.as_ref()
    }

    /// How many ids are held.
    pub(crate) fn live(&self) -> usize {
        self.entries.len() - self.free.len()
    }

    /// The ids held, in id order.
    pub(crate) fn live_ids(&self) -> impl Iterator<Item = u32> + '_ {
        (0u32..)
            .zip(&self.entries)
            .filter_map(|(id, entry)| entry.as_ref().map(|_| id))
    }
}

/// The four slabs scene items name.
#[derive(Default)]
pub(crate) struct Slabs {
    pub(crate) texts: Slab<Rc<Layout>>,
    pub(crate) images: Slab<Rc<RgbaImage>>,
    pub(crate) icons: Slab<Rc<Icon>>,
    pub(crate) surfaces: Slab<String>,
}

/// Which slab an item names an id in, and the id; `None` for an item
/// that names none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Named {
    Text(u32),
    Image(u32),
    Icon(u32),
    Surface(u32),
}

impl Named {
    /// The id `item` names, if any.
    pub(crate) fn of(item: &Item) -> Option<Self> {
        match *item {
            Item::Text {
                text: TextId(id), ..
            } => Some(Self::Text(id)),
            Item::Image {
                image: ImageId(id), ..
            } => Some(Self::Image(id)),
            Item::Icon {
                icon: IconId(id), ..
            } => Some(Self::Icon(id)),
            Item::Surface {
                surface: SurfaceId(id),
                ..
            } => Some(Self::Surface(id)),
            // A shape's entry is the painter's shape store's, not a slab's.
            Item::Shadow(_) | Item::Quad(_) | Item::Shape(_) => None,
        }
    }
}

impl Slabs {
    /// Frees each id `items` name.
    pub(crate) fn free(&mut self, items: &[Item]) {
        for named in items.iter().filter_map(Named::of) {
            match named {
                Named::Text(id) => self.texts.remove("text", id),
                Named::Image(id) => self.images.remove("image", id),
                Named::Icon(id) => self.icons.remove("icon", id),
                Named::Surface(id) => self.surfaces.remove("surface", id),
            }
        }
    }

    /// Every id held, in each slab's id order.
    pub(crate) fn live(&self) -> impl Iterator<Item = Named> + '_ {
        let texts = self.texts.live_ids().map(Named::Text);
        let images = self.images.live_ids().map(Named::Image);
        let icons = self.icons.live_ids().map(Named::Icon);
        texts
            .chain(images)
            .chain(icons)
            .chain(self.surfaces.live_ids().map(Named::Surface))
    }
}

#[cfg(test)]
#[path = "slab_tests.rs"]
mod tests;
