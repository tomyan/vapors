//! Who takes the a11y updates (R6, review-v242 M1): each take clears
//! what the document collected, so a second taker would steal the
//! consumer's changes and nothing would report it. A consumer attaches
//! by name and gets the one [`AxTaker`]; a take by anything else, or a
//! second attach, is refused naming the owner, and a detach consumes
//! the taker and resets the mirror, so a consumer attached later starts
//! with every node.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::a11y_error::AxError;
use crate::a11y_mirror::AxUpdate;
use crate::a11y_mirror_state::AxMirror;
use crate::document::Document;

/// The right to take a document's a11y updates: one per attach, held
/// by the consumer, given back by [`Document::ax_mirror_detach`]. Not
/// cloneable, so the consumer is the one taker.
#[derive(Debug)]
pub struct AxTaker {
    id: u64,
}

/// Ids across every document, so a taker of one document is refused by
/// another.
static ISSUED: AtomicU64 = AtomicU64::new(0);

/// The consumer that owns the takes, if one is attached.
#[derive(Default)]
pub(crate) struct Owner {
    current: Option<(u64, &'static str)>,
}

impl Owner {
    /// Whether `taker` is the owner.
    fn is(&self, taker: &AxTaker) -> bool {
        self.current.is_some_and(|(id, _)| id == taker.id)
    }

    /// Refuses when a consumer owns the takes, naming it.
    pub(crate) fn none(&self) -> Result<(), AxError> {
        match self.current {
            Some((_, owner)) => Err(AxError::Owned {
                owner: owner.to_owned(),
            }),
            None => Ok(()),
        }
    }
}

impl Document {
    /// Attaches a consumer named `owner` (as the fix is named to another
    /// taker, `ElementsApp::ax_update`): the taker every take goes
    /// through until it is detached.
    ///
    /// # Errors
    /// [`AxError::Owned`]: a consumer is attached already.
    pub fn ax_mirror_attach(&mut self, owner: &'static str) -> Result<AxTaker, AxError> {
        self.ax.owner.none()?;
        let id = ISSUED.fetch_add(1, Ordering::Relaxed) + 1;
        self.ax.owner.current = Some((id, owner));
        Ok(AxTaker { id })
    }

    /// The consumer went: the next take holds every node, as a first
    /// take does, and nothing is collected until then.
    ///
    /// # Errors
    /// [`AxError::NotTaker`]: `taker` is not the one attached here.
    pub fn ax_mirror_detach(&mut self, taker: AxTaker) -> Result<(), AxError> {
        if !self.ax.owner.is(&taker) {
            return Err(AxError::NotTaker);
        }
        self.ax = AxMirror::default();
        Ok(())
    }

    /// [`Document::take_ax_update`] by the attached consumer.
    ///
    /// # Errors
    /// [`AxError::NotTaker`]: `taker` is not the one attached here
    /// (another document's, or detached); otherwise as
    /// [`Document::take_ax_update`].
    pub fn take_ax_update_by(&mut self, taker: &AxTaker) -> Result<Option<AxUpdate>, AxError> {
        if !self.ax.owner.is(taker) {
            return Err(AxError::NotTaker);
        }
        self.take_ax_update_unowned()
    }
}
