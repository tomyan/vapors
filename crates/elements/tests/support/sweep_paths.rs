//! The listener-removal sweep's input paths: each drives the page's
//! control one way the platform can, from a fresh page.

#![allow(dead_code)] // each test file uses a subset

use vapors_elements::{Document, Modifiers, TextEdit};

use super::sweep_page::{Page, capture_on_press, the_viewport};

pub type Path = fn(&Page);

pub const PATHS: &[(&str, Path)] = &[
    ("mouse drag", mouse_drag),
    ("mouse drag from the press", press_drag),
    ("captured mouse drag", |p| {
        capture_on_press(p);
        mouse_drag(p);
    }),
    ("click twice", |p| {
        click(p);
        click(p);
    }),
    ("pointer left", |p| {
        Document::pointer_moved(&p.doc, p.point);
        Document::pointer_left(&p.doc);
    }),
    ("press then cancel", |p| {
        Document::pointer_pressed(&p.doc, p.point);
        Document::cancel_pointers(&p.doc);
    }),
    ("touch drag", touch_drag),
    ("captured touch drag", |p| {
        capture_on_press(p);
        touch_drag(p);
    }),
    ("touch cancel", |p| {
        Document::touch_pressed(&p.doc, 1, p.point);
        Document::touch_cancelled(&p.doc, 1, p.point);
    }),
    ("taps", |p| {
        Document::tapped(&p.doc, p.point);
        Document::touch_tapped(&p.doc, p.point);
        Document::touch_double_tapped(&p.doc, p.point);
    }),
    ("keys", |p| {
        focus(p);
        for k in [" ", "Enter", "ArrowUp", "ArrowRight", "Backspace", "Home"] {
            key(p, k);
        }
    }),
    ("space keyup alone", |p| {
        focus(p);
        Document::key_released(&p.doc, " ", Modifiers::default());
    }),
    ("enter keyup alone", |p| {
        focus(p);
        Document::key_released(&p.doc, "Enter", Modifiers::default());
    }),
    ("type then tab", |p| {
        click(p);
        type_five(p);
        tab(p, false);
    }),
    ("type then shift tab", |p| {
        click(p);
        type_five(p);
        tab(p, true);
    }),
    ("type then script blur", |p| {
        focus(p);
        type_five(p);
        Document::focus(&p.doc, None);
    }),
    ("type then window blur", |p| {
        focus(p);
        type_five(p);
        Document::window_focus(&p.doc, false);
        Document::window_focus(&p.doc, true);
    }),
    ("type then settle", |p| {
        focus(p);
        type_five(p);
        Document::settle_focus(&p.doc);
        Document::settle_selection(&p.doc);
    }),
    ("paste", |p| {
        focus(p);
        Document::text_pasted(&p.doc, "7");
        Document::settle_selection(&p.doc);
    }),
    ("platform edit", platform_edit),
    ("tab in and out", |p| {
        tab(p, false);
        tab(p, false);
        tab(p, false);
        tab(p, true);
    }),
];

fn mouse_drag(p: &Page) {
    Document::pointer_moved(&p.doc, p.point);
    press_drag(p);
}

/// Pressed with no move before, so the first `pointermove` is the
/// drag's.
fn press_drag(p: &Page) {
    let [x, y] = p.point;
    Document::pointer_pressed(&p.doc, p.point);
    Document::pointer_moved(&p.doc, [x + 20.0, y]);
    Document::pointer_moved(&p.doc, [x + 40.0, y]);
    Document::pointer_released(&p.doc, [x + 40.0, y]);
}

fn touch_drag(p: &Page) {
    let [x, y] = p.point;
    Document::touch_pressed(&p.doc, 1, p.point);
    Document::touch_moved(&p.doc, 1, [x + 20.0, y]);
    Document::touch_moved(&p.doc, 1, [x + 40.0, y]);
    Document::touch_released(&p.doc, 1, [x + 40.0, y]);
}

fn click(p: &Page) {
    Document::pointer_pressed(&p.doc, p.point);
    Document::pointer_released(&p.doc, p.point);
}

fn focus(p: &Page) {
    Document::focus(&p.doc, Some(p.control));
}

fn key(p: &Page, key: &str) {
    Document::key_pressed(&p.doc, key, Modifiers::default());
    Document::key_released(&p.doc, key, Modifiers::default());
}

fn tab(p: &Page, shift: bool) {
    let modifiers = Modifiers {
        shift,
        ..Modifiers::default()
    };
    Document::key_pressed(&p.doc, "Tab", modifiers);
    Document::key_released(&p.doc, "Tab", modifiers);
}

fn type_five(p: &Page) {
    key(p, "5");
    Document::character_typed(&p.doc, "5");
}

fn platform_edit(p: &Page) {
    focus(p);
    let _ = p.doc.borrow_mut().update(the_viewport());
    let Some(input) = p.doc.borrow_mut().text_input() else {
        return;
    };
    let text = format!("{}9", input.text);
    let end = text.encode_utf16().count();
    let edit = TextEdit {
        before: input.text,
        text,
        selection: (end, end),
        composing: None,
    };
    let _ = Document::text_edited(&p.doc, &edit);
}
