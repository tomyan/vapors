//! What a [`StyleKey`] refuses, each message naming the template site
//! and the fix: a key whose shape a tree cannot keep true
//! ([`StyleKey::shape_refusal`], asked where an element is created
//! from it), an attribute it fixes ([`StyleKey::refusal`]) and a
//! directive it does not list. A tree records the refusal and changes
//! nothing; it never panics for one.

use crate::{MAX_TOGGLES, SCOPE_ATTRIBUTE, StyleKey};

impl StyleKey {
    /// Why `set_attribute` of `name` is refused on an element with this
    /// key: `None` when it is not one the key fixes. Names compare
    /// ASCII-case-insensitively, as HTML's do.
    #[must_use]
    pub fn refusal(&self, name: &str) -> Option<String> {
        let fix = match name.to_ascii_lowercase().as_str() {
            "class" => "toggle a class with `class:name={bool}` there".to_owned(),
            "style" | SCOPE_ATTRIBUTE => "write it there".to_owned(),
            lower if self.carries(lower) => format!("write `{lower}={{expr}}` there"),
            _ => return None,
        };
        Some(format!(
            "`{name}` on <{}> at {} is set from its template; {fix}",
            self.tag, self.site
        ))
    }

    /// Whether `lower` (lowercased) names one of the key's attributes.
    fn carries(&self, lower: &str) -> bool {
        self.attributes
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case(lower))
    }

    /// Why the key cannot describe an element, naming its site and the
    /// fix: `None` when it can. A tree's sharing reads each class and
    /// directive name as one selector class and each directive as one
    /// bit, so a name that is empty or holds whitespace, a directive
    /// listed twice or also a literal class, or more than
    /// [`MAX_TOGGLES`] directives would make it judge wrongly.
    #[must_use]
    pub fn shape_refusal(&self) -> Option<String> {
        let problem = self
            .too_many_toggles()
            .or_else(|| self.malformed_name())
            .or_else(|| self.repeated_directive())
            .or_else(|| self.directive_also_literal())
            .or_else(|| self.attribute_misplaced())?;
        Some(format!(
            "the StyleKey of <{}> at {} {problem}",
            self.tag, self.site
        ))
    }

    /// The refusal for toggling `name`, which the key does not list.
    pub(crate) fn unlisted(&self, name: &str) -> String {
        let listed: Vec<String> = self.toggles.iter().map(|t| format!("`{t}`")).collect();
        let has = if listed.is_empty() {
            "which has no `class:` directives".to_owned()
        } else {
            format!("whose directives are {}", listed.join(", "))
        };
        format!(
            "`class:{name}` is not a directive of <{}> at {}, {has}; add `class:{name}={{bool}}` there",
            self.tag, self.site
        )
    }

    fn too_many_toggles(&self) -> Option<String> {
        let count = self.toggles.len();
        (count > MAX_TOGGLES).then(|| {
            format!("has {count} `class:` directives; at most {MAX_TOGGLES} fit one element, so move some to a child element")
        })
    }

    fn malformed_name(&self) -> Option<String> {
        let names = self.classes.iter().chain(self.toggles);
        let name = names
            .into_iter()
            .find(|name| name.is_empty() || name.chars().any(|c| c.is_ascii_whitespace()))?;
        Some(if name.is_empty() {
            "has an empty class name; drop it".to_owned()
        } else {
            format!(
                "has the class name {name:?}, which holds whitespace; give each class its own entry"
            )
        })
    }

    fn repeated_directive(&self) -> Option<String> {
        let (_, name) = (self.toggles.iter().enumerate())
            .find(|(index, name)| self.toggles[..*index].contains(name))?;
        Some(format!(
            "lists `class:{name}` twice; give each directive once"
        ))
    }

    fn directive_also_literal(&self) -> Option<String> {
        let name = (self.toggles.iter()).find(|name| self.classes.contains(name))?;
        Some(format!(
            "has `{name}` both as a literal class and as `class:{name}`; drop one"
        ))
    }
}

impl StyleKey {
    /// An attribute the key's other fields carry, listed under
    /// `attributes`, or one listed twice: the tree would set it twice
    /// and the analysis read the wrong one.
    fn attribute_misplaced(&self) -> Option<String> {
        for (index, (name, _)) in self.attributes.iter().enumerate() {
            let lower = name.to_ascii_lowercase();
            if matches!(lower.as_str(), "class" | "style" | SCOPE_ATTRIBUTE) {
                return Some(format!(
                    "lists `{name}` under `attributes`, which its own field carries"
                ));
            }
            let again = self.attributes[..index]
                .iter()
                .any(|(earlier, _)| earlier.eq_ignore_ascii_case(name));
            if again {
                return Some(format!("lists attribute `{name}` twice"));
            }
        }
        None
    }
}
