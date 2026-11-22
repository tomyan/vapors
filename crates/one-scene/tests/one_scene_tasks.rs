//! `OneScene` forwards a task wake to its `ElementsApp`: a wrapper that
//! kept the default `tasks_woken` would ask for a frame and never poll.

use std::cell::Cell;
use std::rc::Rc;

use vapors_one_scene::OneScene;
use vapors_platform::{App, Platform};
use vapors_signals::{Deadline, spawn};

#[test]
fn a_task_wake_reaches_the_page_app_through_one_scene() {
    // Given: a task spawned after the scene's ElementsApp installed the hook
    let mut scene = OneScene::new(None).expect("the example mounts");
    let polled = Rc::new(Cell::new(false));
    let seen = Rc::clone(&polled);
    spawn(Deadline::User, async move {
        seen.set(true);
        Ok::<(), String>(())
    });

    // When
    scene.tasks_woken(&Platform::detached());

    // Then
    assert!(
        polled.get(),
        "OneScene::tasks_woken did not reach its ElementsApp"
    );
}
