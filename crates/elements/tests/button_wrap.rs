//! A shrink-to-fit button's text never wraps: the counter demo's
//! "Clicked N times" stays on one line at every N and scale. Taffy
//! offers the text its max-content width back through the button's
//! padding, which can lose an ulp (`vapors_text::fits`).

mod support;

use support::{document, element, text};
use vapors_dom::NodeId;
use vapors_elements::{Document, Viewport};

/// `crates/demos/demos/counter.wisp`'s layout rules.
const COUNTER_CSS: &str = "html, body { height: 100%; }
body { margin: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; }
button { padding: 12px 20px; font-size: 15px; border: none; border-radius: 6px; }
.strip { margin-top: 16px; width: 240px; height: 8px; }";

/// Scales where some N wrapped before the slack, 2.75 among them as
/// on the Pixel 9 Pro (17 wrapped, 19 did not).
const SCALES: [f32; 6] = [1.0, 2.0, 2.625, 2.75, 2.8125, 3.0];

/// The counter's page with `label` in its button, as the compiled
/// component writes it: a text node either side of the number.
fn counter(label: [&str; 3]) -> (Document, NodeId) {
    let mut doc = document();
    doc.add_stylesheet("app.css", COUNTER_CSS).expect("parses");
    let body = doc.body();
    let button = element(&mut doc, body, "button", "");
    for part in label {
        text(&mut doc, button, part);
    }
    element(&mut doc, body, "div", "strip");
    (doc, button)
}

/// The button's height on a 412 by 800 page at `scale`.
fn button_height(label: [&str; 3], scale: f32) -> f32 {
    let (mut doc, button) = counter(label);
    let viewport = Viewport {
        width: 412.0,
        height: 800.0,
        scale,
    };
    let painted = doc.update(viewport).expect("updates");
    painted
        .border_box(button)
        .expect("the button has a box")
        .height
}

#[test]
fn the_counter_button_stays_on_one_line_at_every_count_and_scale() {
    for scale in SCALES {
        // Given: one word, which cannot wrap
        let one_line = button_height(["Clicked", "", ""], scale);

        // When: taller by more than a pixel, as a second line (about
        // 19 px) makes it
        let label = |n: u32| button_height(["Clicked ", &n.to_string(), " times"], scale);
        let wrapped: Vec<u32> = (0..=30).filter(|n| label(*n) - one_line > 1.0).collect();

        // Then
        assert!(
            wrapped.is_empty(),
            "at {scale}, these counts wrapped: {wrapped:?}"
        );
    }
}
