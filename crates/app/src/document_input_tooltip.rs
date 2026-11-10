//! The document's tooltip from the input it is given (`tooltip.rs`
//! holds when it shows): the pointer's title from the element it hits,
//! keyboard focus's from the focused element when a key moved it and
//! it is `:focus-visible`. Every event updates it, the host's too, so
//! a key or press the host takes still hides it. A touch never arms
//! it. A host asks once per frame, after the document's update, for
//! the tooltip shown ([`DocumentInput::tooltip`]) and for when to wake
//! ([`DocumentInput::tooltip_deadline`]).

use vapors_elements::{Document, Point};
use vapors_platform::{Event, Instant};

use super::DocumentInput;
use crate::pointer::Action;
use crate::tooltip::{Titled, Tooltip, TooltipTrigger};

impl DocumentInput {
    /// The tooltip shown at `now`, after checking the one armed or shown
    /// against `document`: the pointer's follows what is under it now
    /// (a removed element or a changed title hides it, and a new title
    /// arms again), a keyboard one hides when its element lost focus or
    /// its title. Ask once per frame, after the update.
    pub fn tooltip(&mut self, document: &Document, now: Instant) -> Option<&Tooltip> {
        match self.tooltip.current().map(|tip| tip.trigger) {
            Some(TooltipTrigger::Pointer) => {
                let target = self.pointer_title(document);
                self.tooltip.pointer_over(target, now);
            }
            Some(TooltipTrigger::Keyboard) if !self.keyboard_tip_holds(document) => {
                self.tooltip.hide();
            }
            _ => {}
        }
        self.tooltip.shown_at(now)
    }

    /// Whether the tooltip armed or shown changed (armed, armed again,
    /// shown or hidden) since last asked. A host that gets `true` after
    /// an event draws a frame, so the hide is drawn and the frame's
    /// `Next` takes the new [`DocumentInput::tooltip_deadline`].
    pub fn take_tooltip_changed(&mut self) -> bool {
        self.tooltip.take_changed()
    }

    /// When the armed tooltip shows: a host waits until then
    /// (`Next::WaitUntil`) and asks again every frame until it shows,
    /// as a frame's `Next::Continue` replaces the wait. `None` when no
    /// tooltip is armed.
    #[must_use]
    pub fn tooltip_deadline(&self) -> Option<Instant> {
        self.tooltip.deadline()
    }

    /// Where the pointer last moved, in CSS pixels: a pointer tooltip's
    /// hotspot ([`crate::ScenePainter::draw_tooltip`]). `None` before
    /// the first move, as the position is then unknown.
    #[must_use]
    pub fn pointer(&self) -> Option<Point> {
        self.pointer
    }

    /// `event`, its pointer `action` if it has one, as the tooltip sees
    /// it, after the document had it.
    pub(super) fn tooltip_event(
        &mut self,
        document: &Document,
        event: &Event,
        action: Option<Action>,
        now: Instant,
    ) {
        if let Some(action) = action {
            self.tooltip_action(document, action, now);
        }
        let key = matches!(event, Event::Key { pressed: true, .. });
        if key || matches!(event, Event::WindowFocus { focused: false }) {
            self.tooltip.hide();
        }
        let target = focus_title(document).filter(|_| key);
        self.tooltip.focus(document.active_element(), target, now);
    }

    fn tooltip_action(&mut self, document: &Document, action: Action, now: Instant) {
        match action {
            Action::Move(point) => self.tooltip.pointer_over(title_at(document, point), now),
            Action::Press(..) => self.tooltip.pressed(),
            Action::Release(..) => self.tooltip.released(),
            Action::Leave => self.tooltip.left(),
            // the wheel and every touch
            _ => self.tooltip.hide(),
        }
    }

    fn pointer_title(&self, document: &Document) -> Option<Titled> {
        title_at(document, self.pointer?)
    }

    /// The keyboard tooltip's element still has focus, its title and a
    /// box (`display: none` hides it).
    fn keyboard_tip_holds(&self, document: &Document) -> bool {
        self.tooltip.current().is_some_and(|tip| {
            document.active_element() == Some(tip.node)
                && has_box(document, tip.node)
                && document.tooltip_text(tip.node) == Some((tip.node, tip.text.clone()))
        })
    }
}

/// Whether the last update gave `node` a box.
fn has_box(document: &Document, node: vapors_dom::NodeId) -> bool {
    document
        .painted()
        .is_some_and(|painted| painted.border_box(node).is_some())
}

/// The title the element hit at `point` shows.
fn title_at(document: &Document, point: vapors_elements::Point) -> Option<Titled> {
    document.tooltip_text(document.hit(point)?)
}

/// The focused element's own title, while it is `:focus-visible`
/// (Chromium's `ElementFocusedFromKeypress` reads the element's title,
/// not an ancestor's).
fn focus_title(document: &Document) -> Option<Titled> {
    let active = document
        .active_element()
        .filter(|_| document.focus_visible())?;
    document
        .tooltip_text(active)
        .filter(|(node, _)| *node == active)
}

#[cfg(test)]
#[path = "document_input_tooltip_keys_tests.rs"]
mod key_tests;
#[cfg(test)]
#[path = "document_input_tooltip_tests.rs"]
mod tests;
