//! Reading the timestamps back carries the map's outcome out: a refused
//! map and one that never reported are failures, each naming its cause.

use std::sync::mpsc::TryRecvError;

use super::mapped;

#[test]
fn a_map_that_succeeded_reads_on() {
    assert_eq!(mapped(Ok(Ok(()))), Ok(()));
}

#[test]
fn a_map_wgpu_refused_fails_with_its_error() {
    let error = mapped(Ok(Err(wgpu::BufferAsyncError))).expect_err("refused");
    assert!(
        error.starts_with("mapping the timestamps for reading failed: "),
        "{error}"
    );
}

#[test]
fn a_map_that_never_reported_fails_naming_the_wait() {
    for outcome in [TryRecvError::Empty, TryRecvError::Disconnected] {
        let error = mapped(Err(outcome)).expect_err("unreported");
        assert!(
            error.contains("had not finished after the GPU wait of 10s"),
            "{error}"
        );
    }
}
