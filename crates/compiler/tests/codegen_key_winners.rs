//! Each element's `StyleKey` points at a `static WINNERS_{VAR}`, its
//! compiled winners (build-time-styles.md, row B2): the sheets it
//! indexes, each with the hash of the text the runtime registers, the
//! always declarations that win, and the rules decided at run time.

use crate::common_codegen as codegen;

use codegen::{assert_has, generated};
use vapors_compiler::styles::CrateSheets;
use vapors_compiler::{PropsIndex, generate_checked, parse_file_under};
use vapors_css::property::PropertyId;
use vapors_css::sheet_hash::sheet_hash;

const STYLE: &str = "\n.card { color: red; }\n.card:hover { color: blue; }\n";

fn styled(template: &str) -> String {
    generated(&format!("<style>{STYLE}</style>\n{template}"))
}

/// A sheet's hash as the generated code writes it.
fn hash(text: &str) -> String {
    let hex = format!("{:016x}", sheet_hash(text));
    format!(
        "0x{}_{}_{}_{}",
        &hex[..4],
        &hex[4..8],
        &hex[8..12],
        &hex[12..]
    )
}

fn longhand(name: &str) -> usize {
    PropertyId::from_name(name).expect("a longhand").index()
}

#[test]
fn an_elements_winners_list_the_user_agent_sheet_and_its_own_by_scope_hashed_as_the_runtime_registers_them()
 {
    // When
    let code = styled("<div class=\"card\"></div>");

    // Then
    assert_has(
        &code,
        "winners: vapors_dom::Winners::Compiled(&WINNERS_DIV0)",
    );
    assert_has(
        &code,
        &format!(
            "sheets: &[(vapors_dom::SheetRef::UserAgent, {}), (vapors_dom::SheetRef::Scope(SCOPE), {})]",
            hash(vapors_css::user_agent::TEXT),
            hash(STYLE)
        ),
    );
}

#[test]
fn an_always_declaration_is_a_base_winner_and_a_state_rule_is_left_to_run_time() {
    // When
    let code = styled("<div class=\"card\"></div>");

    // Then: `.card`'s color, sheet 1 rule 0, wins unless `.card:hover`
    // (sheet 1 rule 1) matches
    assert_has(
        &code,
        &format!(
            "vapors_dom::BaseWinner {{ property: vapors_dom::PropertyRef::Longhand({}), declaration: vapors_dom::DeclarationRef::Rule {{ rule: vapors_dom::RuleRef {{ sheet: 1, rule: 0 }}, declaration: 0 }} }}",
            longhand("color")
        ),
    );
    assert_has(&code, "vapors_dom::RuleRef { sheet: 1, rule: 1 }]");
}

#[test]
fn the_literal_style_wins_as_an_inline_declaration() {
    // When
    let code = styled("<div class=\"card\" style=\"width: 8px; color: green\"></div>");

    // Then
    assert_has(
        &code,
        &format!(
            "vapors_dom::BaseWinner {{ property: vapors_dom::PropertyRef::Longhand({}), declaration: vapors_dom::DeclarationRef::Inline(1) }}",
            longhand("color")
        ),
    );
}

#[test]
fn a_component_tags_key_has_no_winners_as_its_props_type_decides_its_tag() {
    // When
    let code = generated(
        "<script>\nuse vapors_scene3d_components::{Mesh, MeshProps};\n</script>\n<Mesh class=\"body\" />",
    );

    // Then
    assert_has(
        &code,
        "attributes: &[], winners: vapors_dom::Winners::ComponentTag }",
    );
}

#[test]
fn another_components_sheet_that_can_reach_an_element_is_listed_with_its_hash() {
    // Given a crate of two components, the other one styling `div`
    let dir = TempDir::new("key-winners");
    let card = dir.0.join("card.wisp");
    let other = dir.0.join("other.wisp");
    std::fs::write(
        &card,
        format!("<style>{STYLE}</style>\n<div class=\"card\"></div>"),
    )
    .expect("written");
    let other_style = "\n:global(div) { margin: 0; }\n";
    std::fs::write(
        &other,
        format!("<style>{other_style}</style>\n<span></span>"),
    )
    .expect("written");
    let parse = |path: &std::path::Path| parse_file_under(path, &dir.0).expect("parses");
    let (card, other) = (parse(&card), parse(&other));
    let sheets = CrateSheets::of([&card, &other]);

    // When
    let code = generate_checked(&card, &PropsIndex::default(), &sheets.others_for(&card))
        .expect("generates")
        .code;

    // Then: listed third, its rule conditional, since its order against
    // the card's own rules is decided when the sheets are added
    assert_has(
        &code,
        &format!(
            "(vapors_dom::SheetRef::Scope(concat!(env!(\"CARGO_PKG_NAME\"), \"/\", \"other.wisp\")), {})]",
            hash(other_style)
        ),
    );
    assert_has(&code, "vapors_dom::RuleRef { sheet: 2, rule: 0 }]");
}

/// A directory removed when dropped, so a failed test leaves nothing in
/// the system's temporary directory.
struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a temp dir");
        Self(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
