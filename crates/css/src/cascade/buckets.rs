//! Rules bucketed by their subject compound's id, first class or tag, so
//! an element's candidate rules are a few bucket lookups rather than
//! every rule in the sheet.

use std::collections::BTreeMap;

use crate::selector::{Element, Selector};

/// Rule indices by subject key.
#[derive(Default)]
pub(super) struct Buckets {
    ids: BTreeMap<String, Vec<usize>>,
    classes: BTreeMap<String, Vec<usize>>,
    tags: BTreeMap<String, Vec<usize>>,
    universal: Vec<usize>,
}

impl Buckets {
    /// File rule `index` under its subject compound's most specific key.
    pub(super) fn insert(&mut self, selector: &Selector, index: usize) {
        let subject = selector
            .compounds
            .last()
            .expect("a selector has a compound");
        let bucket = if let Some(id) = &subject.id {
            self.ids.entry(id.clone()).or_default()
        } else if let Some(class) = subject.classes.first() {
            self.classes.entry(class.clone()).or_default()
        } else if let Some(tag) = &subject.tag {
            self.tags.entry(tag.clone()).or_default()
        } else {
            &mut self.universal
        };
        bucket.push(index);
    }

    /// Candidate rule indices for `element`, in source order.
    pub(super) fn candidates<E: Element>(&self, element: &E) -> Vec<usize> {
        let mut out = self.universal.clone();
        if let Some(id) = element.id() {
            out.extend(self.ids.get(id).into_iter().flatten());
        }
        for class in element.class_names() {
            out.extend(self.classes.get(class).into_iter().flatten());
        }
        let tag = element.tag_name().to_ascii_lowercase();
        out.extend(self.tags.get(&tag).into_iter().flatten());
        out.sort_unstable();
        out.dedup();
        out
    }
}
