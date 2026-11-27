//! Both switches by keyboard: Space toggles the focused switch on keyup and
//! Enter does not, as a browser checkbox; the tree names each switch as the
//! spec labels it, with its checked state.
use super::support::*;
use super::switches::SWITCHES;
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

#[test]
fn space_toggles_each_focused_switch_on_keyup_and_enter_does_not() {
    for switch in &SWITCHES {
        // Given: focused, off
        let mounted = laid_out();
        Document::focus(&mounted.document, Some(switch.input(&mounted)));

        // When
        press(&mounted, "Enter");
        release(&mounted, "Enter");
        let after_enter = switch.ax(&mounted);
        press(&mounted, " ");
        let space_down = switch.ax(&mounted);
        release(&mounted, " ");

        // Then: off after Enter and while Space is down, on once it is up
        let class = switch.class;
        assert_eq!(
            after_enter,
            switch.expected_ax(false),
            "{class} after Enter"
        );
        assert_eq!(
            space_down,
            switch.expected_ax(false),
            "{class} with Space down"
        );
        assert_eq!(switch.ax(&mounted), switch.expected_ax(true), "{class}");
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
