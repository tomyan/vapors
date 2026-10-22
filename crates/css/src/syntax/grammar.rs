//! The grammar language: text to a tree of nodes.

/// A node of a value grammar.
#[derive(Clone, Debug, PartialEq)]
pub enum Grammar {
    /// A literal keyword.
    Keyword(String),
    /// `<name>` with an optional numeric range.
    Type {
        /// The type name without the angle brackets.
        name: String,
        /// `[lo,hi]` for numeric types; `inf` is unbounded.
        range: Option<(f64, f64)>,
    },
    /// `a | b | c`: the first that matches wins.
    Alternatives(Vec<Grammar>),
    /// `a b c`: in order.
    Sequence(Vec<Grammar>),
    /// `a?`
    Optional(Box<Grammar>),
    /// `a+`, `a*` and `a{lo,hi}`: space-separated repetitions.
    Repeat {
        /// The repeated node.
        node: Box<Grammar>,
        /// Minimum count.
        min: usize,
        /// Maximum count.
        max: usize,
    },
    /// `a#`: comma-separated repetitions, one or more.
    CommaList(Box<Grammar>),
}

/// Parse a grammar.
///
/// # Panics
/// On a malformed grammar: grammars are compile-time constants in the
/// property table, checked by the table's tests.
#[must_use]
pub fn parse_grammar(text: &str) -> Grammar {
    let tokens = tokenize(text);
    let mut pos = 0;
    let g = alternatives(&tokens, &mut pos);
    assert_eq!(
        pos,
        tokens.len(),
        "grammar {text:?}: unexpected {:?}",
        tokens.get(pos)
    );
    g
}

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ' ' => {}
            '<' => {
                let mut name = String::from("<");
                for c in chars.by_ref() {
                    name.push(c);
                    if c == '>' {
                        break;
                    }
                }
                tokens.push(name);
            }
            '{' => {
                let mut rep = String::from("{");
                for c in chars.by_ref() {
                    rep.push(c);
                    if c == '}' {
                        break;
                    }
                }
                tokens.push(rep);
            }
            '|' | '[' | ']' | '?' | '+' | '*' | '#' => tokens.push(c.to_string()),
            other => {
                let mut word = other.to_string();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_alphanumeric() || c == '-' {
                        word.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(word);
            }
        }
    }
    tokens
}

fn alternatives(tokens: &[String], pos: &mut usize) -> Grammar {
    let mut alts = vec![sequence(tokens, pos)];
    while tokens.get(*pos).is_some_and(|t| t == "|") {
        *pos += 1;
        alts.push(sequence(tokens, pos));
    }
    if alts.len() == 1 {
        alts.remove(0)
    } else {
        Grammar::Alternatives(alts)
    }
}

fn sequence(tokens: &[String], pos: &mut usize) -> Grammar {
    let mut terms = Vec::new();
    while let Some(t) = tokens.get(*pos) {
        if t == "|" || t == "]" {
            break;
        }
        terms.push(term(tokens, pos));
    }
    assert!(!terms.is_empty(), "grammar: empty sequence");
    if terms.len() == 1 {
        terms.remove(0)
    } else {
        Grammar::Sequence(terms)
    }
}

fn term(tokens: &[String], pos: &mut usize) -> Grammar {
    let mut node = primary(tokens, pos);
    while let Some(t) = tokens.get(*pos) {
        node = match t.as_str() {
            "?" => Grammar::Optional(Box::new(node)),
            "+" => repeat(node, 1, usize::MAX),
            "*" => repeat(node, 0, usize::MAX),
            "#" => Grammar::CommaList(Box::new(node)),
            rep if rep.starts_with('{') => {
                let body = rep.trim_matches(|c| c == '{' || c == '}');
                let (lo, hi) = body.split_once(',').expect("grammar: {lo,hi}");
                repeat(node, lo.parse().expect("lo"), hi.parse().expect("hi"))
            }
            _ => return node,
        };
        *pos += 1;
    }
    node
}

fn repeat(node: Grammar, min: usize, max: usize) -> Grammar {
    Grammar::Repeat {
        node: Box::new(node),
        min,
        max,
    }
}

fn primary(tokens: &[String], pos: &mut usize) -> Grammar {
    let t = &tokens[*pos];
    *pos += 1;
    if t == "[" {
        let inner = alternatives(tokens, pos);
        assert_eq!(
            tokens.get(*pos).map(String::as_str),
            Some("]"),
            "grammar: unclosed ["
        );
        *pos += 1;
        return inner;
    }
    if let Some(body) = t.strip_prefix('<').and_then(|s| s.strip_suffix('>')) {
        return type_node(body);
    }
    Grammar::Keyword(t.clone())
}

fn type_node(body: &str) -> Grammar {
    let (name, range) = match body.split_once(" [") {
        Some((name, range)) => (name, Some(range.trim_end_matches(']'))),
        None => (body, None),
    };
    let range = range.map(|r| {
        let (lo, hi) = r.split_once(',').expect("grammar: [lo,hi]");
        (bound(lo), bound(hi))
    });
    Grammar::Type {
        name: name.to_owned(),
        range,
    }
}

fn bound(text: &str) -> f64 {
    match text.trim() {
        "inf" => f64::INFINITY,
        "-inf" => f64::NEG_INFINITY,
        n => n.parse().expect("grammar: numeric bound"),
    }
}
