//! The dashboard's two switches for the switch tests: where each is, its
//! state's signal, its name, and what the tree says of it.
use super::support::by_class;
use crate::app::Mounted;
use crate::controls::Controls;
use crate::dashboard::*;
use vapors_dom::NodeId;
use vapors_elements::BoxRect;
use vapors_scene::Rect;
use vapors_signals::Signal;

/// A switch by its class, its box's top left in the window, its name and
/// the signal it binds.
pub struct Switch {
    pub class: &'static str,
    pub x: f32,
    pub y: f32,
    pub label: &'static str,
    pub state: fn(&Controls) -> Signal<bool>,
}

pub const SWITCHES: [Switch; 2] = [
    Switch {
        class: "live-switch",
        x: LIVE_X,
        y: LIVE_Y,
        label: LIVE_LABEL,
        state: |controls| controls.live.on,
    },
    Switch {
        class: "switch",
        x: SWITCH_X,
        y: SWITCH_Y,
        label: COMPARE_LABEL,
        state: |controls| controls.compare,
    },
];

/// The knob's diameter and its distance from the track's edges (the spec's
/// prose: "an 18 px knob 2 px inside").
pub const KNOB: f32 = 18.0;
pub const INSET: f32 = 2.0;

/// What the tree says of a node: role, name and checked state.
pub type AxFacts = (String, String, Option<bool>);

impl Switch {
    pub fn input(&self, mounted: &Mounted) -> NodeId {
        let found = by_class(&mounted.document.borrow(), self.class);
        let [input] = found[..] else {
            panic!(
                "{} elements of class {}, not one; fix SWITCHES in tests/switches.rs (cargo test -p vapors-gui-bench-app switch)",
                found.len(),
                self.class
            )
        };
        input
    }

    /// The app's state for the switch: its signal, which the rest of the
    /// page reads.
    pub fn on(&self, mounted: &Mounted) -> bool {
        (self.state)(&mounted.controls).get_untracked()
    }

    /// Sets the switch's state through its signal, as a frame's state is set.
    pub fn set(&self, mounted: &Mounted, on: bool) {
        (self.state)(&mounted.controls).set(on);
    }

    pub fn inside(&self, rect: Rect) -> bool {
        rect.left >= self.x
            && rect.top >= self.y
            && rect.right <= self.x + SWITCH_WIDTH
            && rect.bottom <= self.y + SWITCH_HEIGHT
    }

    /// The knob's box: 2 px inside the track, at its start or end.
    pub fn knob(&self, on: bool) -> Rect {
        let left = if on {
            self.x + SWITCH_WIDTH - INSET - KNOB
        } else {
            self.x + INSET
        };
        let top = self.y + INSET;
        Rect::new(left, top, left + KNOB, top + KNOB)
    }

    pub fn centre(&self, on: bool) -> [f32; 2] {
        let knob = self.knob(on);
        [knob.left + KNOB / 2.0, knob.top + KNOB / 2.0]
    }

    fn overlaps(&self, b: &BoxRect) -> bool {
        b.x < self.x + SWITCH_WIDTH
            && b.x + b.width > self.x
            && b.y < self.y + SWITCH_HEIGHT
            && b.y + b.height > self.y
    }

    /// The ax nodes no larger than the switch whose bounds overlap its box.
    pub fn ax(&self, mounted: &Mounted) -> Vec<AxFacts> {
        let doc = mounted.document.borrow();
        let mut stack = doc.ax_tree().expect("laid out");
        let mut found = Vec::new();
        while let Some(n) = stack.pop() {
            let small = n.bounds.width <= SWITCH_WIDTH && n.bounds.height <= SWITCH_HEIGHT;
            if small && self.overlaps(&n.bounds) {
                found.push((n.role.clone(), n.name.clone(), n.states.checked));
            }
            stack.extend(n.children);
        }
        found
    }

    /// The one switch node the tree should hold for this switch.
    pub fn expected_ax(&self, on: bool) -> Vec<AxFacts> {
        vec![("switch".into(), self.label.into(), Some(on))]
    }
}
