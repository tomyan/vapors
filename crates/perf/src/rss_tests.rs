//! The resident-set reading: parsed from /proc/self/status text, refused
//! when the line is missing or not a count.

use super::{parse_status_kib, peak_kib, resident_kib};

const STATUS: &str = "Name:\tvapors-perf\nVmPeak:\t  900000 kB\nVmHWM:\t  130000 kB\nVmRSS:\t  123456 kB\nThreads:\t3\n";

#[test]
fn the_vm_rss_line_is_read_in_kib() {
    assert_eq!(parse_status_kib(STATUS, "VmRSS"), Ok(123_456));
}

#[test]
fn the_vm_hwm_line_is_read_as_the_peak() {
    assert_eq!(parse_status_kib(STATUS, "VmHWM"), Ok(130_000));
}

#[test]
fn a_status_without_vm_rss_is_refused_naming_the_file() {
    let error = parse_status_kib("Name:\tx\nThreads:\t3\n", "VmRSS").expect_err("no VmRSS");
    assert!(
        error.contains("/proc/self/status has no VmRSS line"),
        "{error}"
    );
}

#[test]
fn a_vm_rss_that_is_not_a_count_is_refused_naming_the_line() {
    let error = parse_status_kib("VmRSS:\tmuch kB\n", "VmRSS").expect_err("not a count");
    assert!(error.contains("much kB"), "{error}");
}

#[test]
fn a_vm_rss_in_another_unit_is_refused() {
    let error = parse_status_kib("VmRSS:\t12 MB\n", "VmRSS").expect_err("not kB");
    assert!(error.contains("12 MB"), "{error}");
}

#[cfg(target_os = "linux")]
#[test]
fn this_process_has_a_resident_set() {
    let (now, peak) = (resident_kib(), peak_kib());
    assert!(now.expect("readable on Linux") > 0);
    assert!(peak.expect("readable on Linux") > 0);
}

#[cfg(not(target_os = "linux"))]
#[test]
fn another_platform_is_refused_naming_it() {
    let error = resident_kib().expect_err("no /proc");
    assert!(error.contains(std::env::consts::OS), "{error}");
}
