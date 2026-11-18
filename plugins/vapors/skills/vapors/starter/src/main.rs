//! The page: mounts the board, steps the game on a timer, and passes
//! arrow keys to it.

mod game;

include!(concat!(env!("OUT_DIR"), "/wisp.rs"));

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use vapors_app::ElementsApp;
use vapors_elements::Document;
use vapors_platform::{
    App, Event, Frame, Instant, Next, Platform, Start, WindowOptions, app_draws_open,
    hide_status_line, report_error, run,
};
use vapors_text::FontSystem;

/// How often the cars move.
const STEP: Duration = Duration::from_millis(400);

fn main() {
    if let Err(message) = start() {
        report_error(&message);
    }
}

fn start() -> Result<(), String> {
    let document = Rc::new(RefCell::new(Document::new(FontSystem::new())));
    let body = document.borrow().body();
    let board = board::Board::mount(&document, body, board::BoardProps {});
    // the page has no Open button and the status line would cover the board
    app_draws_open();
    hide_status_line();
    let options = WindowOptions {
        app_name: "dodge".into(),
        title: "dodge".into(),
        width: 640,
        height: 480,
    };
    let game = Game {
        inner: ElementsApp::new(document),
        board,
        next_step: Instant::now() + STEP,
    };
    run(options, game).map_err(|e| e.to_string())
}

/// The document's app, plus a clock and the keys. `ElementsApp` alone
/// draws only when the document changes, so a timer needs this wrapper.
struct Game {
    inner: ElementsApp,
    /// Kept for the life of the page: dropping it stops its updates.
    board: board::Board,
    next_step: Instant,
}

impl App for Game {
    fn event(&mut self, event: Event, platform: &Platform) {
        if let Event::Key {
            key, pressed: true, ..
        } = &event
        {
            self.board.world.update(|world| world.key(key));
            platform.request_redraw();
        }
        self.inner.event(event, platform);
    }

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        let now = Instant::now();
        if now >= self.next_step {
            self.board.world.update(game::World::step);
            self.next_step = now + STEP;
        }
        match self.inner.frame(frame) {
            Next::Continue => Next::Continue,
            Next::WaitUntil(at) => Next::WaitUntil(at.min(self.next_step)),
            _ => Next::WaitUntil(self.next_step),
        }
    }

    fn start(&mut self, start: &Start<'_>) -> Result<(), String> {
        self.inner.start(start)
    }

    fn drawn(&mut self, platform: &Platform) {
        self.inner.drawn(platform);
    }
}
