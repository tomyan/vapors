use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};
use vapors_platform::{Event, Instant, Modifiers, Platform, TouchPhase};
use vapors_text::FontSystem;

use crate::DocumentInput;
use crate::tooltip::TOOLTIP_DELAY;

/// Two 100x20 buttons side by side at the top left, the first titled
/// "Open (o)", the second untitled, then a text field titled "Name".
pub(super) fn page() -> (Rc<RefCell<Document>>, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = "body { margin: 0; display: flex; } button, input { width: 100px; height: 20px; }";
    doc.add_stylesheet("app.css", css).expect("parses");
    let body = doc.body();
    let open = doc.create_element("button");
    doc.set_attribute(open, "title", "Open (o)");
    doc.append(body, open);
    let other = doc.create_element("button");
    doc.append(body, other);
    let name = doc.create_element("input");
    doc.set_attribute(name, "title", "Name");
    doc.append(body, name);
    update(&mut doc);
    (Rc::new(RefCell::new(doc)), open)
}

pub(super) fn update(doc: &mut Document) {
    let viewport = Viewport {
        width: 300.0,
        height: 100.0,
        scale: 1.0,
    };
    doc.update(viewport).expect("updates");
}

pub(super) fn give(input: &mut DocumentInput, doc: &Rc<RefCell<Document>>, event: &Event) {
    input
        .give(doc, event, &Platform::detached())
        .expect("placed");
}

pub(super) fn key(name: &str, code: &str) -> Event {
    Event::Key {
        key: name.into(),
        code: code.into(),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::default(),
    }
}

pub(super) fn shown(
    input: &mut DocumentInput,
    doc: &Rc<RefCell<Document>>,
    at: Instant,
) -> Option<String> {
    input.tooltip(&doc.borrow(), at).map(|tip| tip.text.clone())
}

/// The pointer over the titled button, and when its tooltip shows.
fn hovered() -> (Rc<RefCell<Document>>, DocumentInput, Instant) {
    let (doc, _) = page();
    let mut input = DocumentInput::new();
    let before = Instant::now();
    give(&mut input, &doc, &Event::PointerMoved { x: 10.0, y: 10.0 });
    let deadline = input.tooltip_deadline().expect("armed");
    assert!(deadline >= before + TOOLTIP_DELAY);
    (doc, input, deadline)
}

#[test]
fn the_pointer_over_a_title_shows_it_at_the_deadline_and_not_before() {
    // Given
    let (doc, mut input, deadline) = hovered();

    // Then
    assert_eq!(shown(&mut input, &doc, just_before(deadline)), None);
    assert_eq!(shown(&mut input, &doc, deadline), Some("Open (o)".into()));
    assert_eq!(input.tooltip_deadline(), None);
}

#[test]
fn each_hide_trigger_hides_it_and_leaves_no_deadline() {
    let touch = Event::Touch {
        id: 1,
        phase: TouchPhase::Started,
        x: 10.0,
        y: 10.0,
    };
    let triggers = [
        ("leave", Event::PointerLeft),
        ("off the title", Event::PointerMoved { x: 150.0, y: 10.0 }),
        ("press", press(true)),
        ("wheel", Event::Scroll { dx: 0.0, dy: 5.0 }),
        ("Esc", key("Escape", "Escape")),
        ("another key", key("o", "KeyO")),
        ("touch", touch),
        ("blur", Event::WindowFocus { focused: false }),
    ];
    for (name, event) in triggers {
        // Given
        let (doc, mut input, deadline) = hovered();
        let shown_before = shown(&mut input, &doc, deadline).is_some();

        // When
        give(&mut input, &doc, &event);

        // Then
        assert!(shown_before, "{name}");
        assert_eq!(shown(&mut input, &doc, deadline), None, "{name}");
        assert_eq!(input.tooltip_deadline(), None, "{name}");
    }
}

pub(super) fn press(pressed: bool) -> Event {
    Event::PrimaryButton {
        pressed,
        clicks: std::num::NonZeroU32::MIN,
        modifiers: Modifiers::default(),
    }
}

#[test]
fn a_touch_on_a_title_never_arms_it() {
    // Given
    let (doc, _) = page();
    let mut input = DocumentInput::new();

    // When
    for phase in [TouchPhase::Started, TouchPhase::Ended] {
        let event = Event::Touch {
            id: 1,
            phase,
            x: 10.0,
            y: 10.0,
        };
        give(&mut input, &doc, &event);
    }
    give(
        &mut input,
        &doc,
        &Event::Tap {
            x: 10.0,
            y: 10.0,
            clicks: std::num::NonZeroU32::MIN,
        },
    );

    // Then
    assert_eq!(input.tooltip_deadline(), None);
}

#[test]
fn removing_the_element_or_changing_its_title_hides_it() {
    for change in ["remove", "retitle"] {
        // Given
        let (doc, mut input, deadline) = hovered();
        assert!(shown(&mut input, &doc, deadline).is_some());
        let open = doc
            .borrow()
            .tooltip_text(doc.borrow().hit([10.0, 10.0]).expect("hit"));
        let (open, _) = open.expect("titled");

        // When
        if change == "remove" {
            doc.borrow_mut().remove(open);
        } else {
            doc.borrow_mut().set_attribute(open, "title", "Close");
        }
        update(&mut doc.borrow_mut());

        // Then: a new title arms again, from now
        assert_eq!(shown(&mut input, &doc, deadline), None, "{change}");
        let rearmed = input.tooltip_deadline().is_some();
        assert_eq!(rearmed, change == "retitle", "{change}");
    }
}

fn just_before(at: Instant) -> Instant {
    at.checked_sub(Duration::from_nanos(1))
        .expect("after the clock's start")
}

#[test]
fn an_app_that_ignores_the_deadline_shows_it_at_its_next_frame_and_never_early() {
    // Given: an app that never wakes for the deadline, drawing only
    // when something else asks
    let (doc, mut input, deadline) = hovered();
    let early = [Duration::from_millis(400), Duration::from_nanos(1)];

    // When / Then: frames before the deadline show nothing
    for before in early {
        let at = deadline
            .checked_sub(before)
            .expect("after the clock's start");
        assert_eq!(shown(&mut input, &doc, at), None);
        assert_eq!(input.tooltip_deadline(), Some(deadline));
    }

    // When / Then: the first frame after it, however late, shows it
    let late = deadline + Duration::from_secs(3);
    assert_eq!(shown(&mut input, &doc, late), Some("Open (o)".into()));
}
