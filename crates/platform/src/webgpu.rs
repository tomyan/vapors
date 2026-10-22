//! Whether the browser has WebGPU at all. Without `navigator.gpu`,
//! `getContext("webgpu")` returns null and wgpu's surface creation fails
//! with a message about the canvas, which hides the real reason. This
//! check runs first on the web; it is a plain function so the native
//! test suite can pin its answer.

use crate::PlatformError;

/// `Ok` when `navigator.gpu` is present, else the error that names the
/// browsers that have it.
#[cfg_attr(not(target_family = "wasm"), allow(dead_code))] // called on the web; tested here
pub fn required(navigator_gpu_present: bool) -> Result<(), PlatformError> {
    if navigator_gpu_present {
        Ok(())
    } else {
        Err(PlatformError::NoWebGpu(
            "this browser has no navigator.gpu".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_navigator_gpu_is_the_no_webgpu_error_with_the_browser_list() {
        // When
        let error = required(false).expect_err("no WebGPU");

        // Then
        assert!(matches!(error, PlatformError::NoWebGpu(_)));
        assert!(error.to_string().contains("Chrome or Edge 113+"));
    }

    #[test]
    fn a_present_navigator_gpu_passes() {
        // When / Then
        assert!(required(true).is_ok());
    }
}
