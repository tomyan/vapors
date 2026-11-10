//! When a tooltip shows and hides, as Chromium's desktop tooltips do
//! (views' `TooltipController`): it arms when the pointer reaches an
//! element whose title it shows (`Document::tooltip_text`), or
//! keyboard focus that is `:focus-visible` reaches one, and shows
//! [`TOOLTIP_DELAY`] later. A move within the same title does not
//! restart the wait. A press hides it and keeps it hidden until the
//! pointer reaches another title; leaving the window, any key-down,
//! the wheel, a touch and the window's blur hide it. Time is given to
//! every call, so tests use made-up instants.

use std::time::Duration;

use vapors_dom::NodeId;
use vapors_platform::Instant;

/// How long the pointer or focus stays before its tooltip shows:
/// Windows' default initial tooltip delay (`TTM_SETDELAYTIME`), which
/// is also views' `kDefaultShowTooltipDelay`, for the pointer and for
/// keyboard focus alike.
pub const TOOLTIP_DELAY: Duration = Duration::from_millis(500);

/// What brought a tooltip up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TooltipTrigger {
    /// The mouse pointer came to rest over the element.
    Pointer,
    /// Keyboard focus moved to it and is `:focus-visible`.
    Keyboard,
}

/// A tooltip armed or shown: the element whose title it shows, the
/// text as Chromium shows it, and what brought it up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tooltip {
    /// The element with the title.
    pub node: NodeId,
    /// Its title as the tooltip shows it.
    pub text: String,
    /// What brought it up.
    pub trigger: TooltipTrigger,
}

/// An element and the title text it would show.
pub(crate) type Titled = (NodeId, String);

/// The tooltip's state for one document.
#[derive(Default)]
pub(crate) struct TooltipState {
    /// The tooltip armed or shown.
    tip: Option<Tooltip>,
    /// When the armed tooltip shows; `None` once shown.
    deadline: Option<Instant>,
    /// The title the pointer is over, armed for or not.
    hovered: Option<Titled>,
    /// The title a press hid, which does not show again until the
    /// pointer reaches another.
    pressed_on: Option<Titled>,
    button_down: bool,
    /// The focused element last seen, so focus that stays does not
    /// arm again.
    focus_seen: Option<NodeId>,
    /// What is armed or shown changed (an arm, a re-arm, a show or a
    /// hide) since [`TooltipState::take_changed`] last asked.
    changed: bool,
}

impl TooltipState {
    /// The pointer is over `target`'s title (or none): a new title
    /// arms, none hides, and a move within the title the pointer was
    /// over keeps what is armed or shown, a keyboard tooltip included.
    /// While the button is down it hides and nothing else changes, so
    /// a drag ending on the pressed title keeps it hidden.
    pub(crate) fn pointer_over(&mut self, target: Option<Titled>, now: Instant) {
        if self.button_down {
            return self.hide();
        }
        if self.hovered == target && self.tip.is_some() {
            return;
        }
        self.hovered.clone_from(&target);
        if self.pressed_on.is_some() && self.pressed_on == target {
            return self.hide();
        }
        self.pressed_on = None;
        match target {
            Some(target) => self.arm(target, TooltipTrigger::Pointer, now),
            None => self.hide(),
        }
    }

    /// The focused element is `active`; `target` is the title it would
    /// show, given only for focus a key moved that is `:focus-visible`.
    /// Focus that moves hides a keyboard tooltip and arms for `target`.
    pub(crate) fn focus(&mut self, active: Option<NodeId>, target: Option<Titled>, now: Instant) {
        if active == self.focus_seen {
            return;
        }
        self.focus_seen = active;
        if self.trigger() == Some(TooltipTrigger::Keyboard) {
            self.hide();
        }
        if let Some(target) = target {
            self.arm(target, TooltipTrigger::Keyboard, now);
        }
    }

    /// The primary button went down: hidden until the pointer reaches
    /// another title.
    pub(crate) fn pressed(&mut self) {
        self.pressed_on.clone_from(&self.hovered);
        self.button_down = true;
        self.hide();
    }

    /// The primary button came up.
    pub(crate) fn released(&mut self) {
        self.button_down = false;
    }

    /// Hidden, armed or shown, until the next title reached.
    pub(crate) fn hide(&mut self) {
        self.changed |= self.tip.is_some();
        self.tip = None;
        self.deadline = None;
    }

    /// The pointer left the window: hidden, and over no title.
    pub(crate) fn left(&mut self) {
        self.hovered = None;
        self.hide();
    }

    /// When the armed tooltip shows; `None` when none is armed.
    pub(crate) fn deadline(&self) -> Option<Instant> {
        self.deadline
    }

    /// The tooltip armed or shown.
    pub(crate) fn current(&self) -> Option<&Tooltip> {
        self.tip.as_ref()
    }

    /// The tooltip shown at `now`: an armed one shows once `now`
    /// reaches its deadline.
    pub(crate) fn shown_at(&mut self, now: Instant) -> Option<&Tooltip> {
        if self.deadline.is_some_and(|deadline| now >= deadline) {
            self.deadline = None;
            self.changed = true;
        }
        self.tip.as_ref().filter(|_| self.deadline.is_none())
    }

    /// Whether what is armed or shown changed since last asked: a host
    /// draws a frame for it, so a hide is drawn and a new deadline
    /// reaches the frame's `Next`.
    pub(crate) fn take_changed(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }

    fn arm(&mut self, (node, text): Titled, trigger: TooltipTrigger, now: Instant) {
        self.changed = true;
        self.tip = Some(Tooltip {
            node,
            text,
            trigger,
        });
        self.deadline = Some(now + TOOLTIP_DELAY);
    }

    fn trigger(&self) -> Option<TooltipTrigger> {
        self.tip.as_ref().map(|tip| tip.trigger)
    }
}

#[cfg(test)]
#[path = "tooltip_rules_tests.rs"]
mod rules_tests;
#[cfg(test)]
#[path = "tooltip_tests.rs"]
mod tests;
