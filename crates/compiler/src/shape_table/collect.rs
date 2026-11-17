//! The literal shapes of a component's template, in source order.

use vapors_shapes::{FillRule, IconPath, Outline, Paint, RuntimeShape, check_geometry, outline};

use super::Table;
use crate::CompileError;
use crate::ast::{AttrValue, Attributes, Component, Element, Node};
use crate::source::Source;

/// Every literal shape element of `component`, its entries added to `table`.
pub(super) fn elements(component: &Component, table: &mut Table) -> Result<(), CompileError> {
    let source = Source::new(&component.path, &component.text);
    // a site names the file as the generated code does, under the
    // crate's directory, so no path of the build machine is emitted
    let origin = component.site_name();
    let css = component
        .style
        .as_ref()
        .is_some_and(|style| style.text.to_ascii_lowercase().contains("fill"));
    walk(
        &component.template.children,
        false,
        &mut |element, inherited| shapes_of((&source, origin), (element, inherited, css), table),
    )
}

type Visit<'v> = dyn FnMut(&Element, bool) -> Result<(), CompileError> + 'v;

/// `visit` on every element of `nodes`, told whether an `<svg>` above
/// it sets `fill` (which its shapes inherit).
fn walk(nodes: &[Node], inherited: bool, visit: &mut Visit) -> Result<(), CompileError> {
    for node in nodes {
        match node {
            Node::Element(element) => {
                visit(element, inherited)?;
                let sets = element.tag == "svg" && element.attributes.get("fill").is_some();
                walk(&element.children, inherited || sets, visit)?;
            }
            Node::If(block) => {
                let otherwise = block.otherwise.as_deref().unwrap_or_default();
                walk(&block.then, inherited, visit)?;
                walk(otherwise, inherited, visit)?;
            }
            Node::For(block) => walk(&block.children, inherited, visit)?,
            Node::Snippet(snippet) => walk(&snippet.children, inherited, visit)?,
            Node::Component(tag) => walk(&tag.children, inherited, visit)?,
            Node::Text(_) | Node::Render(_) => {}
        }
    }
    Ok(())
}

/// Whether `element` is literally filled. Off, and what the build cannot
/// tell (inherited, `style`, CSS), get no entry: the runtime's used
/// style decides, and S5b falls back to it.
fn filled(element: &Element, inherited: bool, css: bool) -> bool {
    match element.attributes.get("fill") {
        _ if css || element.attributes.get("style").is_some() => false,
        Some(AttrValue::Literal(value)) => !value.trim().eq_ignore_ascii_case("none"),
        Some(AttrValue::Expr(_)) => false,
        None => !inherited,
    }
}

/// `element`'s entries added: none unless it is a shape of literal
/// geometry (bound is a run-time entry, S6) and fill.
fn shapes_of(
    (source, origin): (&Source, &str),
    (element, inherited, css): (&Element, bool, bool),
    table: &mut Table,
) -> Result<(), CompileError> {
    let Some(data) = outline_of(element) else {
        return Ok(());
    };
    let (line, _) = source.position(element.offset);
    let site = format!("{origin}:{line}");
    let path = IconPath::parse(&data).map_err(|error| {
        source.error(
            element.offset,
            format!("<{}> d {data:?} is refused: {error}; fix the path data (docs/icons.md lists what is drawn), then `cargo build` in this crate to check it", element.tag),
        )
    })?;
    if filled(element, inherited, css) {
        let paint = Paint::Fill(FillRule::NonZero);
        add(source, element, (&site, &path, paint), table)?;
    }
    Ok(())
}

fn add(
    source: &Source,
    element: &Element,
    (site, path, paint): (&str, &IconPath, Paint),
    table: &mut Table,
) -> Result<(), CompileError> {
    let refuse = |why: String| {
        source.error(
            element.offset,
            format!(
                "{why}; change the shape at {site}, then `cargo build` in this crate to check it"
            ),
        )
    };
    let shape = RuntimeShape::for_table(site, path, paint).map_err(refuse)?;
    check_geometry(&shape.geometry())
        .map_err(|why| refuse(format!("the shape at {site} cannot be drawn: {why}")))?;
    table.index_of(shape);
    Ok(())
}

/// The path data `element` draws when its geometry is literal: none
/// when it is no shape, its geometry is bound or it draws nothing.
/// Paint attributes are not geometry.
fn outline_of(element: &Element) -> Option<String> {
    let attributes = &element.attributes;
    let Some(names) = vapors_shapes::geometry_names(&element.tag) else {
        return path_data(element);
    };
    let bound = |name: &&str| matches!(attributes.get(name), Some(AttrValue::Expr(_)));
    if names.iter().any(bound) {
        return None;
    }
    match outline(&element.tag, |name| literal(attributes, name)) {
        Ok(Outline::Path(data)) => Some(data),
        Ok(Outline::Nothing) | Err(_) => None,
    }
}

fn path_data(element: &Element) -> Option<String> {
    (element.tag == "path")
        .then(|| literal(&element.attributes, "d").map(str::to_owned))
        .flatten()
}

fn literal<'a>(attributes: &'a Attributes, name: &str) -> Option<&'a str> {
    match attributes.get(name) {
        Some(AttrValue::Literal(value)) => Some(value),
        _ => None,
    }
}
