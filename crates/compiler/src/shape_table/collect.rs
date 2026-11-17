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
    walk(&component.template.children, &mut |element| {
        shapes_of((&source, origin), element, table)
    })
}

type Visit<'v> = dyn FnMut(&Element) -> Result<(), CompileError> + 'v;

fn walk(nodes: &[Node], visit: &mut Visit) -> Result<(), CompileError> {
    for node in nodes {
        match node {
            Node::Element(element) => {
                visit(element)?;
                walk(&element.children, visit)?;
            }
            Node::If(block) => {
                walk(&block.then, visit)?;
                walk(block.otherwise.as_deref().unwrap_or_default(), visit)?;
            }
            Node::For(block) => walk(&block.children, visit)?,
            Node::Snippet(snippet) => walk(&snippet.children, visit)?,
            Node::Component(tag) => walk(&tag.children, visit)?,
            Node::Text(_) | Node::Render(_) => {}
        }
    }
    Ok(())
}

/// `element`'s entries added, none when it is no shape or its geometry
/// is bound (a run-time entry, S6).
fn shapes_of(
    (source, origin): (&Source, &str),
    element: &Element,
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
    if literal(&element.attributes, "fill") != Some("none") {
        add(
            source,
            element,
            (&site, &path, Paint::Fill(FillRule::NonZero)),
            table,
        )?;
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

/// The path data `element` draws when every geometry attribute is a
/// literal. A `<rect>` or `<circle>` the shape checks refuse is left to
/// them (`codegen::basic_shape_attributes`), which name its line.
fn outline_of(element: &Element) -> Option<String> {
    let attributes = &element.attributes;
    if element.tag == "path" {
        return literal(attributes, "d").map(str::to_owned);
    }
    let bound = attributes
        .iter()
        .any(|a| matches!(a.value, AttrValue::Expr(_)));
    if bound || vapors_shapes::allowed(&element.tag).is_none() {
        return None;
    }
    match outline(&element.tag, |name| literal(attributes, name)) {
        Ok(Outline::Path(data)) => Some(data),
        Ok(Outline::Nothing) | Err(_) => None,
    }
}

fn literal<'a>(attributes: &'a Attributes, name: &str) -> Option<&'a str> {
    match attributes.get(name) {
        Some(AttrValue::Literal(value)) => Some(value),
        _ => None,
    }
}
