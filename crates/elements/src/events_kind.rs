//! The one place that says which device a pointer id belongs to: the
//! mouse is `MOUSE`, a finger is `TOUCHES + n`. Every pointer event's
//! `kind` comes from here, so an id and its kind cannot disagree.

use vapors_dom::PointerKind;

use super::capture::{MOUSE, TOUCHES};

/// The kind of pointer `id`. An id no source makes (0, below the
/// touches) is refused loudly, never read as a mouse.
pub(crate) fn kind_of(id: u32) -> PointerKind {
    match id {
        MOUSE => PointerKind::Mouse,
        id if id >= TOUCHES => PointerKind::Touch,
        _ => panic!(
            "pointer id {id} belongs to no device ({}:{}); ids are MOUSE or \
             touch_id(n): fix the caller, reproduce with `CARGO_INCREMENTAL=0 \
             cargo test -p vapors-elements pointer_kind`",
            file!(),
            line!()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_name_their_device() {
        assert_eq!(kind_of(MOUSE), PointerKind::Mouse);
        assert_eq!(kind_of(TOUCHES), PointerKind::Touch);
        assert_eq!(kind_of(TOUCHES + 9), PointerKind::Touch);
    }

    #[test]
    #[should_panic(expected = "belongs to no device")]
    fn an_unplaceable_id_is_refused() {
        kind_of(0);
    }
}
