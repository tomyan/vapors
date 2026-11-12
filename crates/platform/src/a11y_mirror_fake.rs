//! A fake DOM for the mirror's tests: elements by index, each with
//! its attributes, own text, scroll offset and children in order; the
//! writes counted, attributes (text and scroll with them) apart from
//! moves.

use std::collections::BTreeMap;

use super::a11y_mirror_dom::MirrorDom;

#[derive(Default)]
pub(crate) struct FakeElement {
    pub(crate) attributes: BTreeMap<String, String>,
    pub(crate) text: String,
    pub(crate) scroll: [f32; 2],
    pub(crate) children: Vec<usize>,
    pub(crate) parent: Option<usize>,
}

/// Index 0 is the root.
#[derive(Default)]
pub(crate) struct FakeDom {
    pub(crate) elements: Vec<FakeElement>,
    /// Attributes set or removed, texts and scroll offsets written.
    pub(crate) attribute_writes: usize,
    /// Elements appended, inserted or removed.
    pub(crate) moves: usize,
}

impl FakeDom {
    pub(crate) fn with_root() -> Self {
        Self {
            elements: vec![FakeElement::default()],
            attribute_writes: 0,
            moves: 0,
        }
    }

    fn detach(&mut self, child: usize) {
        if let Some(parent) = self.elements[child].parent.take() {
            self.elements[parent].children.retain(|&c| c != child);
        }
    }

    /// The tree under `element` as lines, one per element, indented by
    /// depth: `role name [attr=value ...] "text"`.
    pub(crate) fn lines(&self, element: usize, depth: usize) -> Vec<String> {
        let mut out = vec![self.line(element, depth)];
        for &child in &self.elements[element].children {
            out.extend(self.lines(child, depth + 1));
        }
        out
    }

    fn line(&self, element: usize, depth: usize) -> String {
        let element = &self.elements[element];
        let attrs: Vec<String> = element
            .attributes
            .iter()
            .filter(|(name, _)| !["style", "data-node", "data-scroll"].contains(&name.as_str()))
            .map(|(name, value)| format!("{name}={value}"))
            .collect();
        format!(
            "{}{} \"{}\"",
            "  ".repeat(depth),
            attrs.join(" "),
            element.text
        )
    }
}

impl MirrorDom for FakeDom {
    type Element = usize;

    fn create(&mut self) -> Result<usize, String> {
        self.elements.push(FakeElement::default());
        Ok(self.elements.len() - 1)
    }

    fn set_attribute(&mut self, element: &usize, name: &str, value: &str) {
        self.attribute_writes += 1;
        self.elements[*element]
            .attributes
            .insert(name.to_owned(), value.to_owned());
    }

    fn remove_attribute(&mut self, element: &usize, name: &str) {
        self.attribute_writes += 1;
        self.elements[*element].attributes.remove(name);
    }

    fn set_text(&mut self, element: &usize, text: &str) {
        self.attribute_writes += 1;
        self.elements[*element].text = text.to_owned();
    }

    fn append(&mut self, parent: &usize, child: &usize) {
        self.moves += 1;
        self.detach(*child);
        self.elements[*parent].children.push(*child);
        self.elements[*child].parent = Some(*parent);
    }

    fn insert_before(&mut self, parent: &usize, child: &usize, before: &usize) {
        self.moves += 1;
        self.detach(*child);
        let siblings = &mut self.elements[*parent].children;
        let at = siblings
            .iter()
            .position(|c| c == before)
            .expect("before is a child of parent");
        siblings.insert(at, *child);
        self.elements[*child].parent = Some(*parent);
    }

    fn remove(&mut self, element: &usize) {
        self.moves += 1;
        self.detach(*element);
    }

    fn set_scroll(&mut self, element: &usize, offset: [f32; 2]) {
        self.attribute_writes += 1;
        self.elements[*element].scroll = offset;
    }
}
