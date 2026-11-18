//! Every demo the page shows, in the one order the index lists them
//! and the page's bar steps through: the elements demos this crate
//! mounts ([`all`](crate::all)) and the apps the page runs itself
//! (`crates/web-demo`).

/// Who shows a demo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// An elements demo of this crate, mounted by [`find`](crate::find).
    Elements,
    /// An app the page runs itself: `image`, `one-scene`, `racer`.
    App,
}

/// One demo in the list.
#[derive(Debug, PartialEq, Eq)]
pub struct Listed {
    /// Its name in `?demo=`.
    pub name: &'static str,
    /// What the index and the bar call it.
    pub title: &'static str,
    /// One line on what it shows.
    pub line: &'static str,
    /// Who shows it.
    pub kind: Kind,
}

impl Listed {
    /// The page address that shows it, relative to the page.
    #[must_use]
    pub fn href(&self) -> String {
        format!("?demo={}", self.name)
    }
}

const fn elements(name: &'static str, title: &'static str, line: &'static str) -> Listed {
    Listed {
        name,
        title,
        line,
        kind: Kind::Elements,
    }
}

const fn app(name: &'static str, title: &'static str, line: &'static str) -> Listed {
    Listed {
        name,
        title,
        line,
        kind: Kind::App,
    }
}

/// Every demo, in order.
pub const DEMOS: &[Listed] = &[
    elements("hello", "Hello", "A styled panel with text"),
    elements("counter", "Counter", "A button that counts clicks and taps"),
    elements(
        "boxes",
        "Boxes",
        "Borders, rounded corners, shadows, buttons",
    ),
    elements(
        "shell",
        "Shell",
        "An editor shell, a grid that drops its panel on a phone",
    ),
    elements(
        "panel",
        "Panel",
        "Nested components, each with its own scoped CSS",
    ),
    elements(
        "text",
        "Text",
        "A paragraph of bold, italic and code, and a word you can tap",
    ),
    elements(
        "stress",
        "Stress",
        "1,000 elements, a counter ticking every frame, and how long each update takes",
    ),
    elements(
        "toasts",
        "Toasts",
        "Toasts raised above the shell, each kept until dismissed",
    ),
    elements(
        "scroll",
        "Scroll",
        "Two long lists, each scrolling in its own region by wheel and finger",
    ),
    elements(
        "images",
        "Images",
        "A history list with a thumbnail per turn",
    ),
    elements(
        "cards",
        "Cards",
        "Cards whose contents their parent passes as children",
    ),
    elements(
        "slider",
        "Slider",
        "A range input bound to a signal, dragged and stepped by key",
    ),
    elements(
        "checkbox",
        "Checkbox",
        "Checkboxes bound to signals, toggled by click or Space",
    ),
    elements(
        "number",
        "Number",
        "Number fields, one typed into and one scrubbed, bound to signals",
    ),
    elements(
        "field",
        "Field",
        "A text field bound to a signal, with a caret, a selection and word moves",
    ),
    app("image", "Image", "The image view, pinch and drag"),
    app(
        "one-scene",
        "One scene",
        "HTML over a 3D scene styled by CSS: three cubes turn, night toggles every 15 s, and you click a light to change it",
    ),
    app(
        "racer",
        "Racer",
        "Top-down racing: the platform, GPU rendering and a compiled UI",
    ),
];

/// The demo named `name` in [`DEMOS`].
#[must_use]
pub fn listed(name: &str) -> Option<&'static Listed> {
    DEMOS.iter().find(|demo| demo.name == name)
}

/// The demos before and after `name` in [`DEMOS`], wrapping at the
/// ends: what the page's bar links; `None` for a name not listed.
#[must_use]
pub fn neighbours(name: &str) -> Option<(&'static Listed, &'static Listed)> {
    let at = DEMOS.iter().position(|demo| demo.name == name)?;
    let before = &DEMOS[(at + DEMOS.len() - 1) % DEMOS.len()];
    Some((before, &DEMOS[(at + 1) % DEMOS.len()]))
}

/// Where `key` steps from `name`: `[` to the demo before it, `]` to the
/// one after; `None` for any other key or a name not listed.
#[must_use]
pub fn stepped(name: &str, key: &str) -> Option<&'static Listed> {
    let (before, after) = neighbours(name)?;
    match key {
        "[" => Some(before),
        "]" => Some(after),
        _ => None,
    }
}
