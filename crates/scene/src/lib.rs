//! Scene: the display list the element tree produces and the renderer
//! consumes. This crate has no GPU or window dependency so its contents
//! can be asserted on in tests without a device.

mod color;

pub use color::Color;

#[cfg(test)]
mod proof {
    #[test]
    fn s1_proof_this_test_must_make_ci_red() {
        assert_eq!(1 + 1, 3, "deliberate failure for the S1 proof");
    }
}
