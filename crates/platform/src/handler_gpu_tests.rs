use std::time::Duration;

use super::started_line;

#[test]
fn the_started_line_keeps_the_adapter_line_first() {
    // Given
    let adapter = "adapter: test (Vulkan); max texture 8192 px, max buffer 1024 MiB";

    // When
    let line = started_line(
        adapter,
        Duration::from_millis(12),
        Duration::from_micros(200),
    );

    // Then
    assert!(line.starts_with(adapter), "{line}");
    assert!(
        line.ends_with("start before the first frame 12.0 ms, 0.2 ms after the GPU was ready"),
        "{line}"
    );
}
