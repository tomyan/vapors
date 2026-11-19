//! Who holds keyboard focus on the web page, and when the canvas takes
//! it back. winit hears keys only on the canvas, so focus on `<body>`
//! (nothing focused) leaves the app deaf. The DOM half is
//! `canvas_focus_web`.

// only the web build uses it; native builds run its tests
#![cfg_attr(not(target_family = "wasm"), allow(dead_code))]

/// What holds DOM focus.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum Held {
    /// `<body>`, the page's root, or nothing: keys reach no listener.
    Body,
    /// The canvas.
    Canvas,
    /// A focusable element the page owns, or the text bridge's textarea.
    Other,
}

/// Whether the canvas should take focus now: only from `<body>`, and
/// only while the browser window has focus itself. A window that lost
/// focus (another app, another tab) is left alone, so focus is where it
/// was when the window comes back.
pub(crate) fn takes_focus(held: Held, window_focused: bool) -> bool {
    held == Held::Body && window_focused
}

/// What failed in the DOM half.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Failed {
    /// `canvas.focus()` threw.
    Focus,
    /// `set_timeout` threw.
    Timeout,
    /// There is no `window` when the listener runs.
    NoWindow,
}

/// The report for `failed`: what failed, what the canvas does instead,
/// and the fix. `detail` is the browser's error, when there is one.
pub(crate) fn failure_message(failed: Failed, detail: &str) -> String {
    match failed {
        Failed::Focus => format!(
            "cannot focus the canvas ({detail}); keys may not reach the app until \
             the user clicks it. Fix: give the canvas a tabindex and keep it in the document"
        ),
        Failed::Timeout => format!(
            "cannot schedule the focus check ({detail}); the canvas does not take focus \
             back after this event. Fix: allow timers on the page (check its CSP and sandbox)"
        ),
        Failed::NoWindow => "there is no window to schedule the focus check; the canvas \
             does not take focus back after this event. Fix: run the app in a page, \
             not a worker"
            .to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Failed, Held, failure_message, takes_focus};

    #[test]
    fn each_failure_says_what_instead_and_the_fix() {
        for (f, what) in [
            (Failed::Focus, "cannot focus the canvas (E)"),
            (Failed::Timeout, "cannot schedule the focus check (E)"),
            (Failed::NoWindow, "no window"),
        ] {
            let m = failure_message(f, "E");
            assert!(m.contains(what), "{m}");
            assert!(m.contains("; "), "{m}");
            assert!(m.contains("Fix: "), "{m}");
        }
    }

    #[test]
    fn body_gives_focus_to_the_canvas() {
        assert!(takes_focus(Held::Body, true));
    }

    #[test]
    fn an_element_the_page_owns_keeps_focus() {
        assert!(!takes_focus(Held::Other, true));
    }

    #[test]
    fn the_canvas_has_nothing_to_take() {
        assert!(!takes_focus(Held::Canvas, true));
    }

    #[test]
    fn a_window_that_lost_focus_is_left_alone() {
        assert!(!takes_focus(Held::Body, false));
    }
}
