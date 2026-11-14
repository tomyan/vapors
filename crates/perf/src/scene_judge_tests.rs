//! The 60 s run's refusals: a frame over budget, a device without
//! timestamps, a load average too high or unread.

use std::time::Duration;

use super::{load_from, over_budget, quiet, require_hardware, require_timestamps};
use crate::sampler::JudgedFrame;

fn frame(index: usize, idle_micros: u64) -> JudgedFrame {
    JudgedFrame {
        index,
        idle: Duration::from_micros(idle_micros),
        ..JudgedFrame::default()
    }
}

#[test]
fn a_frame_over_16_7_ms_to_idle_is_over_budget_and_one_at_it_is_not() {
    // Given
    let frames = [frame(0, 16_700), frame(1, 16_701), frame(2, 3_000)];

    // When
    let over: Vec<usize> = over_budget(&frames).iter().map(|f| f.index).collect();

    // Then
    assert_eq!(over, [1]);
}

#[test]
fn a_device_without_timestamp_queries_is_refused_naming_it() {
    let error = require_timestamps(wgpu::Features::empty(), "llvmpipe").expect_err("refused");
    assert!(error.contains("llvmpipe"), "{error}");
    assert!(error.contains("TIMESTAMP_QUERY"), "{error}");
}

#[test]
fn a_device_with_timestamps_but_not_inside_encoders_is_accepted_as_the_timer_is() {
    assert_eq!(
        require_timestamps(wgpu::Features::TIMESTAMP_QUERY, "a WebGPU adapter"),
        Ok(())
    );
}

#[test]
fn a_device_with_timestamp_queries_is_accepted() {
    assert_eq!(
        require_timestamps(crate::gpu_time::FEATURES, "Intel UHD 630"),
        Ok(())
    );
}

#[test]
fn the_load_is_the_first_field_of_proc_loadavg() {
    let line = Ok("1.25 0.80 0.60 2/900 12345\n".to_owned());
    assert_eq!(load_from(line), Ok(1.25));
}

#[test]
fn an_unread_or_unparsable_load_is_refused_saying_why() {
    let unread = load_from(Err(std::io::Error::other("gone"))).expect_err("unread");
    assert!(unread.contains("gone"), "{unread}");
    let garbled = load_from(Ok("busy\n".to_owned())).expect_err("garbled");
    assert!(garbled.contains("busy"), "{garbled}");
}

#[test]
fn a_load_of_1_5_or_more_is_refused_printing_it_and_less_is_accepted() {
    let error = quiet("start", Ok(1.5)).expect_err("too busy");
    assert!(error.contains("1.50 at the start"), "{error}");
    assert_eq!(quiet("end", Ok(1.49)), Ok(1.49));
    assert!(quiet("end", Err("unread".into())).is_err());
}

#[test]
fn only_an_integrated_or_discrete_gpu_is_accepted_and_every_other_type_is_refused_by_name() {
    use wgpu::DeviceType::{Cpu, DiscreteGpu, IntegratedGpu, Other, VirtualGpu};
    let cases = [
        (IntegratedGpu, None),
        (DiscreteGpu, None),
        (Cpu, Some("is Cpu: a software adapter")),
        (VirtualGpu, Some("is VirtualGpu: not known to be a GPU")),
        (Other, Some("is Other: not known to be a GPU")),
    ];
    for (device_type, refused) in cases {
        // When
        let checked = require_hardware(device_type, "an adapter", "a driver");

        // Then
        match refused {
            None => assert_eq!(checked, Ok(()), "{device_type:?}"),
            Some(why) => {
                let error = checked.expect_err("refused");
                assert!(
                    error.starts_with(&format!("an adapter (a driver) {why}")),
                    "{error}"
                );
            }
        }
    }
}
