//! The page's bar steps through `DEMOS` in its order: its links are the
//! demos before and after, wrapping at the ends, and `[` and `]` step
//! to them.

use vapors_demos::{neighbours, stepped};

type Pair = Option<(&'static vapors_demos::Listed, &'static vapors_demos::Listed)>;

fn names(pair: Pair) -> Option<(&'static str, &'static str)> {
    pair.map(|(before, after)| (before.name, after.name))
}

#[test]
fn the_demos_before_and_after_wrap_at_the_ends_of_the_list() {
    assert_eq!(names(neighbours("hello")), Some(("racer", "counter")));
    assert_eq!(names(neighbours("counter")), Some(("hello", "boxes")));
    assert_eq!(names(neighbours("racer")), Some(("one-scene", "hello")));
}

#[test]
fn a_name_not_listed_has_no_neighbours() {
    assert_eq!(names(neighbours("index")), None);
    assert_eq!(names(neighbours("wake")), None);
}

#[test]
fn brackets_step_back_and_on_and_no_other_key_steps() {
    let name = |key| stepped("counter", key).map(|demo| demo.name);
    assert_eq!(name("["), Some("hello"));
    assert_eq!(name("]"), Some("boxes"));
    assert_eq!(name("ArrowRight"), None);
    assert_eq!(stepped("nope", "]"), None);
}
