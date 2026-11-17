use super::{Counted, Scan};
use crate::api_prices::Usage;

fn scanned(chunks: &[&str]) -> Counted {
    let mut scan = Scan::default();
    for chunk in chunks {
        scan.read(chunk.as_bytes());
    }
    scan.counted()
}

const START: &str = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"model\":\"claude-sonnet-5-5\",\
    \"usage\":{\"input_tokens\":10,\"cache_creation_input_tokens\":200,\"cache_read_input_tokens\":3000,\"output_tokens\":1,\
    \"cache_creation\":{\"ephemeral_5m_input_tokens\":200,\"ephemeral_1h_input_tokens\":0},\"service_tier\":\"standard\"}}}\n\n";
const DELTA: &str = "data: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":57}}\n\n";
const STOP: &str = "data: {\"type\":\"message_stop\"}\n\n";

#[test]
fn a_whole_stream_counts_its_input_and_its_last_output_for_the_model_billed() {
    // Given: the events split across reads mid-line
    let (first, second) = START.split_at(40);

    // Then
    let usage = Usage {
        input: 10,
        output: 57,
        cache_write: 200,
        cache_read: 3000,
    };
    assert_eq!(
        scanned(&[first, second, DELTA, STOP]),
        Counted::Whole(usage, "claude-sonnet-5-5".into())
    );
}

#[test]
fn a_stream_cut_before_message_stop_is_not_counted_at_its_first_numbers() {
    let cut = scanned(&[START]);
    assert_eq!(
        cut,
        Counted::Cut("a stream that ended before message_stop".into())
    );
}

#[test]
fn a_body_that_is_not_a_stream_counts_its_usage() {
    let body = "{\"model\":\"m\",\"usage\":{\"input_tokens\":4,\"output_tokens\":9}}";
    let usage = Usage {
        input: 4,
        output: 9,
        ..Usage::default()
    };
    assert_eq!(scanned(&[body]), Counted::Whole(usage, "m".into()));
}

#[test]
fn usage_the_prices_cannot_express_is_not_counted() {
    for (field, value) in [
        ("cache_creation", "{\"ephemeral_1h_input_tokens\":5}"),
        ("server_tool_use", "{\"web_search_requests\":1}"),
        ("speed", "\"fast\""),
        ("inference_geo", "\"us\""),
        ("a_new_field", "1"),
    ] {
        let body =
            format!("{{\"model\":\"m\",\"usage\":{{\"input_tokens\":4,\"{field}\":{value}}}}}");
        let counted = scanned(&[&body]);
        assert!(
            matches!(&counted, Counted::Unpriced(why) if why.contains(field)),
            "{field}: {counted:?}"
        );
    }
}

#[test]
fn a_second_message_start_in_one_response_is_not_counted() {
    let twice = scanned(&[START, START, DELTA, STOP]);
    assert_eq!(twice, Counted::Unpriced("a second message_start".into()));
}
