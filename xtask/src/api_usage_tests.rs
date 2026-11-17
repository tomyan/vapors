use super::Scan;
use crate::api_prices::Usage;

fn scanned(chunks: &[&str]) -> Option<Usage> {
    let mut scan = Scan::default();
    for chunk in chunks {
        scan.read(chunk.as_bytes());
    }
    scan.usage()
}

#[test]
fn a_stream_counts_message_starts_input_and_the_last_deltas_output() {
    // Given: the events split across reads mid-line
    let start = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"usage\":\
                 {\"input_tokens\":10,\"cache_creation_input_tokens\":200,\"cache_read_input_tokens\":3000,\
                 \"output_tokens\":1}}}\n\n";
    let (first, second) = start.split_at(40);
    let delta = "data: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":57}}\n\n";

    // Then
    let usage = Usage {
        input: 10,
        output: 57,
        cache_write: 200,
        cache_read: 3000,
    };
    assert_eq!(scanned(&[first, second, delta]), Some(usage));
}

#[test]
fn a_body_that_is_not_a_stream_counts_its_usage() {
    let body = "{\"usage\":{\"input_tokens\":4,\"output_tokens\":9}}";
    let usage = Usage {
        input: 4,
        output: 9,
        ..Usage::default()
    };
    assert_eq!(scanned(&[body]), Some(usage));
}

#[test]
fn a_response_with_no_usage_is_uncountable_not_free() {
    assert_eq!(scanned(&["{\"type\":\"error\"}"]), None);
    assert_eq!(scanned(&["data: {\"type\":\"ping\"}\n"]), None);
}
