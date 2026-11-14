//! `wisp-check`'s note on each `<Mesh>` whose `color` no rule of its
//! file's `<style>` that can match it, nor its own literal `style`,
//! declares (docs/plans/css-for-3d.md C3, risk 2). Such a mesh takes its
//! colour from the HTML around the scene, so a page's `body { color }`
//! tints it. Rules of other sheets (the app's, another crate's) are not
//! seen, so this is a note, never a refusal. A mesh is a component tag
//! whose last path segment is `Mesh`: the tag its props declare
//! (`MeshProps::ANCHOR_TAG`) is known only to rustc, so an alias
//! (`Mesh as Hull`) is not noted.

use vapors_css::cascade::Cascade;
use vapors_css::property::longhands::COLOR;
use vapors_css::property::scene::SceneTag;

use crate::CompileError;
use crate::ast::{AttrValue, Component, ComponentTag, Node};
use crate::source::Source;
use vapors_css::ast::Declaration;

/// The notes for `component`, each as `file:line:col: note: ...`.
///
/// # Errors
/// The `<style>` does not parse, which `parse_file` refuses first.
pub fn scene_notes(component: &Component) -> Result<Vec<String>, CompileError> {
    let source = Source::new(&component.path, &component.text);
    let cascade = cascade_of(component, &source)?;
    let mut meshes = Vec::new();
    collect(&component.template.children, &mut meshes);
    Ok((meshes.into_iter())
        .filter(|tag| !coloured(&cascade, tag))
        .map(|tag| note(&source, tag))
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

/// Every `<Mesh>` tag in `nodes`, blocks and children included.
fn collect<'a>(nodes: &'a [Node], found: &mut Vec<&'a ComponentTag>) {
    for node in nodes {
        match node {
            Node::Component(tag) => {
                found.extend(is_mesh(tag).then_some(tag));
                collect(&tag.children, found);
            }
            Node::Element(element) => collect(&element.children, found),
            Node::If(block) => {
                collect(&block.then, found);
                collect(block.otherwise.as_deref().unwrap_or(&[]), found);
            }
            Node::For(block) => collect(&block.children, found),
            Node::Snippet(snippet) => collect(&snippet.children, found),
            Node::Text(_) | Node::Render(_) => {}
        }
    }
}

fn is_mesh(tag: &ComponentTag) -> bool {
    let last = tag.name.rsplit("::").next().unwrap_or_default();
    SceneTag::from_tag(&last.to_ascii_lowercase()) == Some(SceneTag::Mesh)
}

/// Whether a rule that can match `tag`'s anchor, or its literal `style`,
/// declares `color`.
fn coloured(cascade: &Cascade, tag: &ComponentTag) -> bool {
    let mut classes = Vec::new();
    for attribute in tag.attributes.iter() {
        match (attribute.name.strip_prefix("class:"), &attribute.value) {
            (Some(toggle), _) => classes.push(toggle),
            (None, AttrValue::Literal(text)) if attribute.name == "class" => {
                classes.extend(text.split_whitespace());
            }
            _ => {}
        }
    }
    let mesh = SceneTag::Mesh.name();
    let by_rule =
        (cascade.rules_for_key(mesh, &classes)).any(|rule| declares_colour(&rule.declarations));
    by_rule || literal_style(tag).is_some_and(|style| declares_colour(&style))
}

/// The tag's literal `style`, parsed; `None` without one or when it
/// does not parse (the runtime refuses that).
fn literal_style(tag: &ComponentTag) -> Option<Vec<Declaration>> {
    vapors_css::parse_inline(tag.attributes.get("style")?.literal()?).ok()
}

fn declares_colour(declarations: &[Declaration]) -> bool {
    (declarations.iter()).any(|d| d.longhands.iter().any(|longhand| longhand.id == COLOR))
}

fn note(source: &Source, tag: &ComponentTag) -> String {
    let (line, col) = source.position(tag.offset);
    format!(
        "{}:{line}:{col}: note: <{}> takes its `color` from the HTML around the scene: no rule of this file that can match it, nor its `style`, declares `color`, so a page's `body {{ color }}` tints it (docs/plans/css-for-3d.md 1.10); declare one if that is not meant",
        source.path, tag.name
    )
}
