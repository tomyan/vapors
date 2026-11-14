//! The user-agent sheet (`src/ua.css`, `src/user_agent.rs`) parses
//! here, so an edit this crate refuses fails this crate's tests rather
//! than every document's construction in vapors-elements.

use vapors_css::user_agent;

#[test]
fn the_user_agent_sheet_parses_and_has_rules() {
    // When
    let sheet = user_agent::sheet();

    // Then
    assert_ne!(sheet.rules.len(), 0);
    assert_eq!(
        vapors_css::parse(user_agent::TEXT).expect("the same text"),
        sheet
    );
}
