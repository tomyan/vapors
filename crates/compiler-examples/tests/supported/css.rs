//! The CSS sections: properties from the parser's table, and the
//! selectors, at-rules, units and functions it accepts, found by parsing
//! each candidate from CSS's own lists.

use vapors_css::property::scene::SceneTag;
use vapors_css::property::{IGNORED_VENDOR_PROPERTIES, PROPERTIES, PropertyId, SHORTHANDS};

use crate::candidates::{AT_RULES, COLOURS, LENGTH_UNITS, PSEUDO_CLASSES, WIDTH_FUNCTIONS};
use crate::codes;

pub fn section() -> String {
    [
        properties(),
        shorthands(),
        selectors(),
        at_rules(),
        values(),
        scene_tags(),
    ]
    .join("\n")
}

fn parses(css: &str) -> bool {
    vapors_css::parse(css).is_ok()
}

fn accepted<'a>(candidates: &[&'a str], wrap: impl Fn(&str) -> String) -> Vec<&'a str> {
    candidates
        .iter()
        .copied()
        .filter(|each| parses(&wrap(each)))
        .collect()
}

fn properties() -> String {
    let rows = PROPERTIES
        .iter()
        .map(|def| {
            let inherited = if def.inherited { "yes" } else { "no" };
            format!(
                "| `{}` | {inherited} | `{}` |\n",
                def.name,
                def.syntax.replace('|', "\\|")
            )
        })
        .collect::<Vec<_>>()
        .concat();
    format!(
        "## CSS properties\n\nThe parser's table: each property, whether it inherits, and its \
         grammar. Custom properties (`--name`) are accepted too.\n\n\
         | Property | Inherited | Grammar |\n|---|---|---|\n{rows}"
    )
}

fn shorthands() -> String {
    let rows = SHORTHANDS
        .iter()
        .map(|(name, longhands)| format!("- `{name}`: {}\n", codes(longhands.iter().copied())))
        .collect::<Vec<_>>()
        .concat();
    let ignored = IGNORED_VENDOR_PROPERTIES
        .iter()
        .map(|(name, why)| format!("- `{name}`: {why}\n"))
        .collect::<Vec<_>>()
        .concat();
    format!(
        "## Shorthands\n\nEach sets these longhands:\n\n{rows}\n\
         Vendor-prefixed properties are refused, except these, which are dropped:\n\n{ignored}"
    )
}

fn selectors() -> String {
    let pseudo = accepted(PSEUDO_CLASSES, |name| format!("a:{name} {{}}"));
    let refused: Vec<&str> = PSEUDO_CLASSES
        .iter()
        .copied()
        .filter(|p| !pseudo.contains(p))
        .collect();
    format!(
        "## Selectors\n\nType, `*`, `.class`, `#id`, attribute selectors and the combinators \
         (descendant, `>`, `+`, `~`). Pseudo-classes accepted: {}.\n\nRefused: {}, and every \
         pseudo-element (`::before`, `::after`).\n",
        codes(pseudo.iter().map(|p| p.split('(').next().unwrap_or(p))),
        codes(refused.iter().map(|p| p.split('(').next().unwrap_or(p))),
    )
}

fn at_rules() -> String {
    let (yes, no): (Vec<&str>, Vec<&str>) = AT_RULES.iter().partition(|rule| parses(rule));
    format!(
        "## At-rules\n\nAccepted: {}.\n\nRefused: {}.\n",
        codes(yes.iter().map(|rule| at_rule_name(rule))),
        codes(no.iter().map(|rule| at_rule_name(rule))),
    )
}

/// `@media` of `@media (..) { .. }`.
fn at_rule_name(rule: &str) -> &str {
    rule.split([' ', ';']).next().unwrap_or(rule)
}

fn values() -> String {
    let units = accepted(LENGTH_UNITS, |unit| format!("a {{ width: 1{unit}; }}"));
    let functions = accepted(WIDTH_FUNCTIONS, |f| format!("a {{ width: {f}; }}"));
    let colours = accepted(COLOURS, |c| format!("a {{ color: {c}; }}"));
    let refused: Vec<&str> = COLOURS
        .iter()
        .copied()
        .filter(|c| !colours.contains(c))
        .collect();
    format!(
        "## Values\n\nLength units: {}.\n\nLength functions: {}.\n\nColours: {}.\n\nRefused \
         colours: {}.\n",
        codes(units),
        codes(functions),
        codes(colours),
        codes(refused),
    )
}

fn scene_tags() -> String {
    let rows = SceneTag::ALL
        .iter()
        .map(|tag| {
            let read = codes(
                PropertyId::all()
                    .filter(|id| tag.reads(*id))
                    .map(PropertyId::name),
            );
            let read = if read.is_empty() {
                "none".to_owned()
            } else {
                read
            };
            format!("- `{}`: {read}\n", tag.name())
        })
        .collect::<Vec<_>>()
        .concat();
    format!(
        "## Scene tags\n\nThe 3D scene components (`<Scene>`, `<Group>`, `<Mesh>`, `<Camera>`) \
         are styled through these tags, and each reads only these properties:\n\n{rows}"
    )
}
