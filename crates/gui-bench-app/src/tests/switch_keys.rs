//! Both switches by keyboard: Space toggles the focused switch on keyup and
//! Enter does not, as a browser checkbox; the tree names each switch as the
//! spec labels it, with its checked state.
use super::support::*;
use super::switches::SWITCHES;
use crate::app::Mounted;
use vapors_elements::{Document, Modifiers};

fn key(mounted: &Mounted, key: &str) {
    let doc = &mounted.document;
    Document::key_pressed(doc, key, Modifiers::default());
    Document::key_released(doc, key, Modifiers::default());
    refresh(mounted);
}

#[test]
fn space_toggles_each_focused_switch_and_enter_does_not() {
    for switch in &SWITCHES {
        // Given: focused, off
        let mounted = laid_out();
        Document::focus(&mounted.document, Some(switch.input(&mounted)));

        // When
        key(&mounted, "Enter");
        let after_enter = switch.ax(&mounted);
        key(&mounted, " ");

        // Then
        assert_eq!(after_enter, switch.expected_ax(false), "{}", switch.class);
        assert_eq!(
            switch.ax(&mounted),
            switch.expected_ax(true),
            "{}",
            switch.class
        );
    }
}

#[test]
fn the_tree_names_each_switch_as_the_spec_labels_it() {
    for switch in &SWITCHES {
        // Given, When
        let mounted = laid_out();

        // Then
        assert_eq!(
            switch.ax(&mounted),
            switch.expected_ax(false),
            "{}",
            switch.class
        );
    }
}
