//! Pastes whose clipboard answers arrive out of order: each answer goes
//! with the field it was read for, so a read refused at once (one is
//! still in progress) does not take the place of a slow one ahead of it.
#![cfg(not(target_family = "wasm"))]

#[path = "support/paste_page.rs"]
mod paste_page;

use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use paste_page::{click_at, click_field, field};
use vapors_app::ElementsApp;
use vapors_dom::Dom;
use vapors_elements::Viewport;
use vapors_platform::{
    App, Clipboard, ClipboardDone, ClipboardImage, ClipboardText, Event, Platform,
};

/// A clipboard whose first read answers slowly and whose second is
/// refused at once, as `SystemClipboard` refuses a read while one is in
/// progress.
struct Overtaken {
    reads: u32,
    /// Raised once the slow read has answered.
    answered: Arc<AtomicBool>,
}

impl Clipboard for Overtaken {
    fn image(&mut self) -> ClipboardImage {
        ClipboardImage::NoImage
    }

    fn read_text(&mut self, done: ClipboardDone<ClipboardText>) {
        self.reads += 1;
        if self.reads == 1 {
            let answered = Arc::clone(&self.answered);
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(200));
                done(ClipboardText::Text("th".to_owned()));
                answered.store(true, Ordering::SeqCst);
            });
        } else {
            done(ClipboardText::Unreadable(
                "a clipboard read is still in progress".to_owned(),
            ));
        }
    }
}

#[test]
fn an_answer_that_overtakes_another_does_not_take_its_field() {
    // Given: a second field below the first; a paste in the first, then
    // one in the second refused before the first answers
    let (doc, first) = field();
    let second = {
        let mut doc = doc.borrow_mut();
        let body = doc.body();
        let second = doc.create_element("input");
        doc.append(body, second);
        doc.update(Viewport {
            width: 200.0,
            height: 100.0,
            scale: 1.0,
        })
        .expect("updates");
        second
    };
    let answered = Arc::default();
    let clipboard = Overtaken {
        reads: 0,
        answered: Arc::clone(&answered),
    };
    let mut app = ElementsApp::new(Rc::clone(&doc)).with_clipboard(Box::new(clipboard));
    let platform = Platform::detached();
    click_field(&mut app, &platform);
    app.event(Event::Paste, &platform);
    click_at(&mut app, &platform, [150.0, 30.0]);
    assert_eq!(doc.borrow().active_element(), Some(second));
    app.event(Event::Paste, &platform);

    // When: the first answers (5 s, 25 times its 200 ms)
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !answered.load(Ordering::SeqCst) {
        assert!(
            std::time::Instant::now() < deadline,
            "the first paste never answered"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    app.deliver_pastes();

    // Then: its text was for the first field, which lost focus, so it
    // went nowhere; paired by arrival it would land in the second
    assert_eq!(doc.borrow().value(second).as_deref(), Some(""));
    assert_eq!(doc.borrow().value(first).as_deref(), Some("Warm"));
}
