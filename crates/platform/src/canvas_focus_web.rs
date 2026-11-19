//! The canvas holds keyboard focus on the web (`canvas_focus`): it takes
//! it at start unless the page's own element has it, and takes it back
//! when it would land on `<body>`: a `focusout` from the canvas or the
//! text bridge's textarea, or a click on an element that cannot hold it
//! (`#status`, `#error`). The check runs after the event, when
//! `document.activeElement` is settled, so a focusable element the
//! press moved focus to is left alone.

use wasm_bindgen::prelude::*;
use web_sys::{Document, Event, HtmlCanvasElement};

use crate::PlatformError;
use crate::canvas_focus::{Failed, Held, failure_message, takes_focus};

/// Focus the canvas, unless the page's element holds focus, and keep it
/// off `<body>` for the page's life.
///
/// # Errors
/// The page has no document or canvas, or a listener cannot be added.
pub(crate) fn install() -> Result<(), PlatformError> {
    let fail = |what: String| PlatformError::Windowing(format!("the canvas focus: {what}"));
    let canvas = crate::web::canvas()?;
    // winit sets this when it makes the window; without a tabindex
    // `focus()` does nothing, so the platform does not rely on that
    if !canvas.has_attribute("tabindex") {
        canvas
            .set_attribute("tabindex", "0")
            .map_err(|e| fail(format!("{e:?}")))?;
    }
    let document = web_sys::window()
        .and_then(|w| w.document())
        .ok_or_else(|| fail("no document".into()))?;
    take(&document, &canvas, true);
    for name in ["focusout", "click"] {
        listen(&document, &canvas, name).map_err(|e| fail(format!("{name}: {e:?}")))?;
    }
    Ok(())
}

/// On `name`, check after the event (a timeout of 0) who holds focus.
fn listen(document: &Document, canvas: &HtmlCanvasElement, name: &str) -> Result<(), JsValue> {
    let (doc, canvas) = (document.clone(), canvas.clone());
    let listener = Closure::<dyn FnMut(Event)>::new(move |_event: Event| {
        let (doc, canvas) = (doc.clone(), canvas.clone());
        let later = Closure::once_into_js(move || take(&doc, &canvas, false));
        schedule(&later);
    });
    document.add_event_listener_with_callback(name, listener.as_ref().unchecked_ref())?;
    // the page has no teardown: the listener lives as long as it does
    listener.forget();
    Ok(())
}

/// Run `later` after the event; report when it cannot.
fn schedule(later: &JsValue) {
    let Some(window) = web_sys::window() else {
        crate::report::report_error(&failure_message(Failed::NoWindow, ""));
        return;
    };
    if let Err(e) = window.set_timeout_with_callback(later.unchecked_ref()) {
        crate::report::report_error(&failure_message(Failed::Timeout, &format!("{e:?}")));
    }
}

/// Focuses the canvas if `takes_focus` says so; `at_start` skips the
/// window's own focus, which a page just loaded may not have yet.
fn take(document: &Document, canvas: &HtmlCanvasElement, at_start: bool) {
    let focused = at_start
        || document.has_focus().unwrap_or_else(|e| {
            crate::report::report_error(&format!(
                "cannot tell whether the window has focus ({e:?}); the canvas keeps what it has"
            ));
            false
        });
    if takes_focus(held(document, canvas), focused)
        && let Err(e) = canvas.focus()
    {
        crate::report::report_error(&failure_message(Failed::Focus, &format!("{e:?}")));
    }
}

fn held(document: &Document, canvas: &HtmlCanvasElement) -> Held {
    match document.active_element() {
        None => Held::Body,
        Some(active) if active.is_same_node(Some(canvas)) => Held::Canvas,
        Some(active) if matches!(active.tag_name().as_str(), "BODY" | "HTML") => Held::Body,
        Some(_) => Held::Other,
    }
}
