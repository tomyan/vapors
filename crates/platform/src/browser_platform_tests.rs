use super::*;

const MAC_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)";

fn mac_agent() -> String {
    MAC_AGENT.to_owned()
}

#[test]
fn the_hinted_platform_is_used_when_the_browser_names_one() {
    assert_eq!(
        chosen(Some("macOS".to_owned()), || Some(mac_agent())),
        "macOS"
    );
}

#[test]
fn an_empty_or_missing_hinted_platform_falls_to_the_user_agent() {
    assert_eq!(chosen(Some(String::new()), || Some(mac_agent())), MAC_AGENT);
    assert_eq!(chosen(None, || Some(mac_agent())), MAC_AGENT);
}

#[test]
fn with_neither_the_platform_is_empty() {
    assert_eq!(chosen(None, || None), "");
}
