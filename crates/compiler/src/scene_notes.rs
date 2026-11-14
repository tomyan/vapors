//! `wisp-check`'s note on each `<Mesh>` whose `color` nothing in its
//! file declares: no rule of the file's `<style>` that can match the
//! mesh or an HTML element of the template around it, and not the
//! mesh's literal `style` (docs/plans/css-for-3d.md C3, risk 2). Such a
//! mesh takes its colour from the HTML around it outside this file, so
//! a page's `body { color }` tints it. `<Scene>` and `<Group>` anchors
//! cannot declare `color` (section 4.2 refuses it), so they are not
//! looked at. A `color` of `inherit`, `currentcolor` or `unset` only
//! passes the parent's on, so it does not count. Rules of other sheets
//! (the app's, another crate's) are not seen, so this is a note, never
//! a refusal. A mesh is a component tag whose last path segment is
//! `Mesh`: the tag its props declare (`MeshProps::ANCHOR_TAG`) is known
//! only to rustc, so an alias (`Mesh as Hull`) is not noted.

use vapors_css::ast::{ComponentValue, Declaration};
use vapors_css::cascade::Cascade;
use vapors_css::property::longhands::COLOR;
use vapors_css::property::scene::SceneTag;
use vapors_css::token::TokenKind;

use crate::CompileError;
use crate::ast::{AttrValue, Attributes, Component, ComponentTag, Node};
use crate::source::Source;

/// What a rule matches on: a tag and its attributes.
type Keyed<'a> = (&'a str, &'a Attributes);

/// A `<Mesh>` and the HTML elements of the template around it.
struct Found<'a> {
    mesh: &'a ComponentTag,
    around: Vec<Keyed<'a>>,
}

/// The notes for `component`, each as `file:line:col: note: ...`.
///
/// # Errors
/// The `<style>` does not parse, which `parse_file` refuses first.
pub fn scene_notes(component: &Component) -> Result<Vec<String>, CompileError> {
    let source = Source::new(&component.path, &component.text);
    let cascade = cascade_of(component, &source)?;
    let mut found = Vec::new();
    collect(&component.template.children, &mut Vec::new(), &mut found);
    Ok((found.into_iter())
        .filter(|found| !coloured(&cascade, found))
        .map(|found| note(&source, found.mesh))
        .collect())
}

/// The component's own `<style>`, scoped as the runtime adds it.
fn cascade_of(component: &Component, source: &Source) -> Result<Cascade, CompileError> {
    let Some(style) = &component.style else {
        return Ok(Cascade::new());
    };
    let sheet = vapors_css::parse(&style.text)
        .map_err(|error| crate::style_check::positioned(source, style, error))?;
    let scope = component.scope.as_deref().unwrap_or_default();
    let sheet = vapors_css::scope::scoped(sheet, crate::SCOPE_ATTRIBUTE, scope);
    Ok(Cascade::new().author(sheet))
}

/// Every `<Mesh>` in `nodes`, blocks and children included, with the
/// elements `around` it.
fn collect<'a>(nodes: &'a [Node], around: &mut Vec<Keyed<'a>>, found: &mut Vec<Found<'a>>) {
    for node in nodes {
        match node {
            Node::Component(tag) => {
                if is_mesh(tag) {
                    found.push(Found {
                        mesh: tag,
                        around: around.clone(),
                    });
                }
                collect(&tag.children, around, found);
            }
            Node::Element(element) => {
                around.push((&element.tag, &element.attributes));
                collect(&element.children, around, found);
                around.pop();
            }
            Node::If(block) => {
                collect(&block.then, around, found);
                collect(block.otherwise.as_deref().unwrap_or(&[]), around, found);
            }
            Node::For(block) => collect(&block.children, around, found),
            Node::Snippet(snippet) => collect(&snippet.children, around, found),
            Node::Text(_) | Node::Render(_) => {}
        }
    }
}

fn is_mesh(tag: &ComponentTag) -> bool {
    let last = tag.name.rsplit("::").next().unwrap_or_default();
    SceneTag::from_tag(&last.to_ascii_lowercase()) == Some(SceneTag::Mesh)
}

/// Whether the file declares `found`'s colour: on the mesh, by a rule
/// or its literal `style`, or on an element around it.
fn coloured(cascade: &Cascade, found: &Found<'_>) -> bool {
    let mesh = (SceneTag::Mesh.name(), &found.mesh.attributes);
    let by_rule = |&(tag, attributes): &Keyed<'_>| {
        let classes = classes(attributes);
        (cascade.rules_for_key(tag, &classes)).any(|rule| declares_colour(&rule.declarations))
    };
    let by_style = literal_style(&found.mesh.attributes).is_some_and(|d| declares_colour(&d));
    by_rule(&mesh) || by_style || found.around.iter().any(by_rule)
}

/// The literal classes and every `class:` directive's name.
fn classes(attributes: &Attributes) -> Vec<&str> {
    let mut classes = Vec::new();
    for attribute in attributes.iter() {
        match (attribute.name.strip_prefix("class:"), &attribute.value) {
            (Some(toggle), _) => classes.push(toggle),
            (None, AttrValue::Literal(text)) if attribute.name == "class" => {
                classes.extend(text.split_whitespace());
            }
            _ => {}
        }
    }
    classes
}

/// The literal `style`, parsed; `None` without one or when it does not
/// parse (the runtime refuses that).
fn literal_style(attributes: &Attributes) -> Option<Vec<Declaration>> {
    vapors_css::parse_inline(attributes.get("style")?.literal()?).ok()
}

/// A declaration of `color` that is not only the parent's passed on.
fn declares_colour(declarations: &[Declaration]) -> bool {
    let colour = |d: &&Declaration| d.longhands.iter().any(|longhand| longhand.id == COLOR);
    (declarations.iter())
        .filter(colour)
        .any(|d| !passes_parents_on(&d.value))
}

/// `inherit`, `currentcolor` (on `color`, the parent's) or `unset`.
fn passes_parents_on(value: &[ComponentValue]) -> bool {
    let [ComponentValue::Token(token)] = value else {
        return false;
    };
    matches!(&token.kind, TokenKind::Ident(name)
        if ["inherit", "currentcolor", "unset"].contains(&name.to_ascii_lowercase().as_str()))
}

fn note(source: &Source, tag: &ComponentTag) -> String {
    let (line, col) = source.position(tag.offset);
    format!(
        "{}:{line}:{col}: note: <{}> takes its `color` from outside this file: no rule here that can match it or an element around it, nor its `style`, declares `color`, so a page's `body {{ color }}` tints it (docs/plans/css-for-3d.md 1.10); declare one if that is not meant",
        source.path, tag.name
    )
}
