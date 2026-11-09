use super::*;

#[test]
fn a_browser_on_macos_iphone_or_ipad_uses_command_and_others_control() {
    // Given: `navigator.userAgentData.platform` values and user agents
    let cases = [
        ("macOS", true),
        ("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)", true),
        (
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)",
            true,
        ),
        ("Mozilla/5.0 (iPad; CPU OS 17_0 like Mac OS X)", true),
        ("Windows", false),
        ("Linux", false),
        ("Chrome OS", false),
        ("Mozilla/5.0 (X11; CrOS x86_64 14541.0.0)", false),
        // iPadOS Safari in desktop mode names itself a Mac
        (
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) Version/17.0 Safari/605.1.15",
            true,
        ),
        ("Mozilla/5.0 (Linux; Android 14; Pixel 9 Pro)", false),
        ("", false),
    ];

    for (platform, command) in cases {
        // When
        let answer = names_apple(platform);

        // Then
        assert_eq!(answer, command, "{platform:?}");
    }
}
