//! Splits a file into its `<script>` and `<style>` sections and the
//! template that follows them.

use crate::CompileError;
use crate::ast::Section;
use crate::source::Source;

pub(crate) struct Sections {
    pub script: Option<Section>,
    pub style: Option<Section>,
    pub template_start: usize,
    pub template_end: usize,
}

/// Sections come first, in either order, each at most once; the rest
/// of the file is the template.
pub(crate) fn split(source: &Source) -> Result<Sections, CompileError> {
    let mut sections = Sections {
        script: None,
        style: None,
        template_start: 0,
        template_end: 0,
    };
    let mut pos = 0;
    let mut last_end = 0;
    loop {
        let start = pos + whitespace_len(&source.text[pos..]);
        let rest = &source.text[start..];
        if rest.starts_with("<script") {
            let section = take(source, start, "script")?;
            put(source, &mut sections.script, section, "script")?;
        } else if rest.starts_with("<style") {
            let section = take(source, start, "style")?;
            put(source, &mut sections.style, section, "style")?;
        } else {
            break;
        }
        pos = tail_end(sections.script.as_ref(), sections.style.as_ref(), start);
        last_end = pos;
    }
    sections.template_start = pos + whitespace_len(&source.text[pos..]);
    sections.template_end = source.text.trim_end().len();
    if sections.template_start >= sections.template_end {
        return Err(source.error(
            last_end,
            "a .wisp needs a template: put at least one element after the <script> and <style> sections",
        ));
    }
    Ok(sections)
}

fn whitespace_len(text: &str) -> usize {
    text.len() - text.trim_start().len()
}

/// The end of whichever section starts at `start`.
fn tail_end(script: Option<&Section>, style: Option<&Section>, start: usize) -> usize {
    [script, style]
        .into_iter()
        .flatten()
        .find(|section| section.offset > start)
        .map_or(start, Section::end)
}

fn take(source: &Source, start: usize, name: &str) -> Result<Section, CompileError> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    if !source.text[start..].starts_with(&open) {
        return Err(source.error(
            start,
            format!("the <{name}> tag takes no attributes: write exactly `{open}`"),
        ));
    }
    let content_start = start + open.len();
    let Some(found) = source.text[content_start..].find(&close) else {
        return Err(source.error(
            start,
            format!("the <{name}> section is never closed: add `{close}` after its content"),
        ));
    };
    Ok(Section {
        text: source.text[content_start..content_start + found].to_string(),
        offset: content_start,
        close_len: close.len(),
    })
}

fn put(
    source: &Source,
    slot: &mut Option<Section>,
    section: Section,
    name: &str,
) -> Result<(), CompileError> {
    if slot.is_some() {
        let open_len = name.len() + 2;
        return Err(source.error(
            section.offset - open_len,
            format!("a .wisp has at most one <{name}> section; merge this one into the first"),
        ));
    }
    *slot = Some(section);
    Ok(())
}
