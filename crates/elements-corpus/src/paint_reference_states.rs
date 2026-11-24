//! The changed states of paint fixtures that `gpu_paint_rules` holds the
//! paint rules to: CSS appended to a fixture's style the way a vapors bug
//! would change its rendering (a radius, a shadow, a border width, a
//! 1 px shift). Chromium's rendering of each is a saved reference
//! (`paint_reference`), as the fixtures as they are.

/// One changed state: `css` appended to `fixture`'s style.
#[derive(Debug, PartialEq, Eq)]
pub struct State {
    /// Its name in the reference's file and the capture command.
    pub name: &'static str,
    /// The paint fixture it changes.
    pub fixture: &'static str,
    /// The CSS appended to the fixture's style.
    pub css: &'static str,
}

impl State {
    /// The state `name` of `fixture`.
    ///
    /// # Errors
    /// When there is no state `name`, or it changes another fixture.
    pub fn of(fixture: &str, name: &str) -> Result<&'static Self, String> {
        let state = STATES.iter().find(|s| s.name == name).ok_or_else(|| {
            format!(
                "no state {name} (paint_reference_states.rs STATES); fix: name one of {:?}",
                STATES.iter().map(|s| s.name).collect::<Vec<_>>()
            )
        })?;
        if state.fixture != fixture {
            return Err(format!(
                "state {name} changes {}, not {fixture}; fix: name {} with it",
                state.fixture, state.fixture
            ));
        }
        Ok(state)
    }
}

const fn state(name: &'static str, fixture: &'static str, css: &'static str) -> State {
    State { name, fixture, css }
}

/// Every changed state the paint rules are checked against.
pub const STATES: &[State] = &[
    state(
        "card_radius_4",
        "paint_boxes",
        ".card { border-radius: 4px; }",
    ),
    state(
        "card_radius_6",
        "paint_boxes",
        ".card { border-radius: 6px; }",
    ),
    state(
        "card_moved_1px",
        "paint_boxes",
        ".card { margin-left: 1px; }",
    ),
    state(
        "unshadowed_radius_6",
        "paint_boxes",
        ".soft { border-radius: 6px; }",
    ),
    state(
        "lone_corner_30",
        "paint_boxes",
        ".corner { border-top-left-radius: 30px; }",
    ),
    state(
        "unscaled_mixed_radii",
        "paint_boxes",
        ".mixed { border-radius: 20px; }",
    ),
    state(
        "shadow_blur_8",
        "paint_boxes",
        ".card { box-shadow: 0 4px 8px #00000059; }",
    ),
    state(
        "shadow_offset_2",
        "paint_boxes",
        ".card { box-shadow: 0 2px 12px #00000059; }",
    ),
    state(
        "spread_3",
        "paint_boxes",
        ".spread { box-shadow: 0 0 0 3px #3a7bd5; }",
    ),
    state(
        "one_border_side_7",
        "paint_boxes",
        ".sides { border-left-width: 7px; width: 81px; }",
    ),
    state(
        "mute_joins_bevelled",
        "icons_paint",
        ".mute path { stroke-linejoin: bevel; }",
    ),
    state(
        "icon_moved_1px",
        "icons_paint",
        ".expand svg { position: relative; left: 1px; }",
    ),
    state(
        "icon_scaled_to_18px",
        "icons_paint",
        ".export svg { width: 18px; height: 18px; }",
    ),
    state(
        "doubled_stroke_width",
        "icons_paint",
        ".voice path { stroke-width: 3; }",
    ),
];
