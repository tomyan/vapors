//! Both switches by keyboard: Space toggles the focused switch on keyup and
//! Enter does not, as a browser checkbox, in the tree and in the app's
//! signal; the tree names each switch as the spec labels it, with its
//! checked state.
use super::support::*;
use super::switches::{AxFacts, SWITCHES, Switch};
use crate::app::Mounted;
use vapors_elements::{Document, Modifiers};

fn press(mounted: &Mounted, key: &str) {
    Document::key_pressed(&mounted.document, key, Modifiers::default());
    refresh(mounted);
}

fn release(mounted: &Mounted, key: &str) {
    Document::key_released(&mounted.document, key, Modifiers::default());
    refresh(mounted);
}

/// What the tree says of `switch` and the app's signal for it.
fn facts(mounted: &Mounted, switch: &Switch) -> (Vec<AxFacts>, bool) {
    (switch.ax(mounted), switch.on(mounted))
}

#[test]
fn space_toggles_each_focused_switch_on_keyup_and_enter_does_not() {
    for switch in &SWITCHES {
        // Given: focused, off
        let mounted = laid_out();
        Document::focus(&mounted.document, Some(switch.input(&mounted)));
        let (off, on) = (
            (switch.expected_ax(false), false),
            (switch.expected_ax(true), true),
        );

        // When
        press(&mounted, "Enter");
        release(&mounted, "Enter");
        let after_enter = facts(&mounted, switch);
        press(&mounted, " ");
        let space_down = facts(&mounted, switch);
        release(&mounted, " ");

        // Then: off after Enter and while Space is down, on once it is up,
        // in the tree and in the app's signal
        let class = switch.class;
        assert_eq!(after_enter, off, "{class} after Enter");
        assert_eq!(space_down, off, "{class} with Space down");
        assert_eq!(facts(&mounted, switch), on, "{class}");
    }
}

#[test]
fn the_tree_names_each_switch_as_the_spec_labels_it() {
    for switch in &SWITCHES {
        // Given, When
        let mounted = laid_out();

        // Then
        let class = switch.class;
        assert_eq!(switch.ax(&mounted), switch.expected_ax(false), "{class}");
    }
}
