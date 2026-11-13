//! What a kept text or icon batch's items resolved to when its bytes
//! were built (`painter_kept_text.rs`): a text id freed by a repaint can
//! be issued to other text (IU4 risk 8), so kept bytes are current only
//! while each item resolves to the same layout or icon, compared by
//! identity.

use std::rc::Rc;

use vapors_elements::Painted;
use vapors_scene::{Item, Run};
use vapors_text::{Icon, Layout};

/// The layouts or icons a batch's items named, kept alive so that no
/// other can take their place in memory.
pub(super) enum Resolved {
    Texts(Vec<Rc<Layout>>),
    Icons(Vec<Rc<Icon>>),
}

impl Resolved {
    /// What `run`'s items resolve to in `painted`.
    pub(super) fn of(run: &Run, painted: &Painted) -> Self {
        match run {
            Run::Texts(items) => Self::Texts(
                items
                    .iter()
                    .map(|i| Rc::clone(text_of(painted, i)))
                    .collect(),
            ),
            Run::Icons(items) => Self::Icons(
                items
                    .iter()
                    .map(|i| Rc::clone(icon_of(painted, i)))
                    .collect(),
            ),
            other => unreachable!("{other:?} is not a text or icon run"),
        }
    }

    /// True when `run`'s items resolve in `painted` to the same layouts
    /// or icons, by identity.
    pub(super) fn matches(&self, run: &Run, painted: &Painted) -> bool {
        match (self, run) {
            (Self::Texts(kept), Run::Texts(items)) => same(kept, items, |i| text_of(painted, i)),
            (Self::Icons(kept), Run::Icons(items)) => same(kept, items, |i| icon_of(painted, i)),
            _ => false,
        }
    }
}

/// `kept` and what each of `items` resolves to are the same allocations.
fn same<'a, T: 'a>(kept: &[Rc<T>], items: &[Item], resolve: impl Fn(&Item) -> &'a Rc<T>) -> bool {
    kept.len() == items.len()
        && kept
            .iter()
            .zip(items)
            .all(|(kept, item)| Rc::ptr_eq(kept, resolve(item)))
}

fn text_of<'a>(painted: &'a Painted, item: &Item) -> &'a Rc<Layout> {
    match *item {
        Item::Text { text, .. } => painted.text(text),
        ref other => unreachable!("a text run holds {other:?}"),
    }
}

fn icon_of<'a>(painted: &'a Painted, item: &Item) -> &'a Rc<Icon> {
    match *item {
        Item::Icon { icon, .. } => painted.icon(icon),
        ref other => unreachable!("an icon run holds {other:?}"),
    }
}
