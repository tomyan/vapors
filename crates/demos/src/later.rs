//! The demos of F2c S8 on and of the edit UI's steps, and the index,
//! which is last.

use crate::mounts::register_thumbnails;
use crate::{Demo, INDEX, cards, images, index, scroll, toasts};

/// Their demos, in slice order.
pub(crate) fn slices() -> Vec<Demo> {
    vec![
        Demo {
            name: "toasts",
            pixels: Some(include_str!("../demos/toasts.pixels")),
            mount: |dom, parent| {
                std::mem::forget(toasts::Toasts::mount(dom, parent, toasts::ToastsProps {}));
                None
            },
        },
        Demo {
            name: "scroll",
            pixels: Some(include_str!("../demos/scroll.pixels")),
            mount: |dom, parent| {
                std::mem::forget(scroll::Scroll::mount(dom, parent, scroll::ScrollProps {}));
                None
            },
        },
        Demo {
            name: "images",
            pixels: Some(include_str!("../demos/images.pixels")),
            mount: |dom, parent| {
                register_thumbnails(&mut dom.borrow_mut());
                std::mem::forget(images::Images::mount(dom, parent, images::ImagesProps {}));
                None
            },
        },
        Demo {
            name: "cards",
            pixels: Some(include_str!("../demos/cards.pixels")),
            mount: |dom, parent| {
                register_thumbnails(&mut dom.borrow_mut());
                std::mem::forget(cards::Cards::mount(dom, parent, cards::CardsProps {}));
                None
            },
        },
        Demo {
            name: INDEX,
            pixels: None,
            mount: |dom, parent| {
                std::mem::forget(index::Index::mount(dom, parent, index::IndexProps {}));
                None
            },
        },
    ]
}
