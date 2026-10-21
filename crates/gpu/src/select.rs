//! The selection rule, on plain data so it can be tested without a GPU.

use std::fmt;

use crate::ADAPTER_ENV;

/// What selection needs to know about one adapter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// The adapter's name as the driver reports it.
    pub name: String,
    /// True for CPU rasterisers such as llvmpipe.
    pub software: bool,
    /// Backend name, for the report.
    pub backend: String,
}

/// Nothing matched. `Display` says what was wanted, what was found, and
/// what to do.
#[derive(Debug, PartialEq, Eq)]
pub struct NoAdapter {
    found: Vec<String>,
    wanted: Option<String>,
}

impl NoAdapter {
    pub(crate) fn new(found: Vec<String>, wanted: Option<&str>) -> Self {
        Self {
            found,
            wanted: wanted.map(str::to_owned),
        }
    }
}

impl fmt::Display for NoAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let wanted = match &self.wanted {
            Some(text) => format!("an adapter whose name contains \"{text}\" ({ADAPTER_ENV})"),
            None => "any adapter".to_owned(),
        };
        let found = if self.found.is_empty() {
            "none".to_owned()
        } else {
            self.found.join("; ")
        };
        write!(
            f,
            "no GPU adapter: wanted {wanted}; found {found}. \
             Fix: install a Vulkan driver (mesa-vulkan-drivers gives lavapipe on Linux), \
             check VK_ICD_FILENAMES if set, or set {ADAPTER_ENV} to a name in the found list."
        )
    }
}

impl std::error::Error for NoAdapter {}

/// Index of the chosen candidate.
///
/// # Errors
///
/// [`NoAdapter`] when `wanted` matches nothing, or there are no candidates.
pub fn choose(candidates: &[Candidate], wanted: Option<&str>) -> Result<usize, NoAdapter> {
    let chosen = match wanted {
        Some(text) => {
            let text = text.to_lowercase();
            candidates
                .iter()
                .position(|c| c.name.to_lowercase().contains(&text))
        }
        None => candidates
            .iter()
            .position(|c| !c.software)
            .or((!candidates.is_empty()).then_some(0)),
    };
    chosen.ok_or_else(|| NoAdapter::new(describe_all(candidates), wanted))
}

fn describe_all(candidates: &[Candidate]) -> Vec<String> {
    candidates
        .iter()
        .map(|c| {
            let kind = if c.software { "software" } else { "hardware" };
            format!("{} ({kind}, {})", c.name, c.backend)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intel() -> Candidate {
        Candidate {
            name: "Intel(R) UHD Graphics 630".into(),
            software: false,
            backend: "Vulkan".into(),
        }
    }

    fn llvmpipe() -> Candidate {
        Candidate {
            name: "llvmpipe (LLVM 19.1.7, 256 bits)".into(),
            software: true,
            backend: "Vulkan".into(),
        }
    }

    #[test]
    fn unset_prefers_hardware_over_software_regardless_of_order() {
        // Given
        let candidates = [llvmpipe(), intel()];

        // When
        let index = choose(&candidates, None).expect("something to choose");

        // Then
        assert_eq!(index, 1);
    }

    #[test]
    fn unset_falls_back_to_software_when_that_is_all_there_is() {
        // Given
        let candidates = [llvmpipe()];

        // When
        let index = choose(&candidates, None).expect("software is acceptable");

        // Then
        assert_eq!(index, 0);
    }

    #[test]
    fn wanted_matches_a_substring_ignoring_case() {
        // Given
        let candidates = [intel(), llvmpipe()];

        // When
        let index = choose(&candidates, Some("LLVMPIPE")).expect("matches");

        // Then
        assert_eq!(index, 1);
    }

    #[test]
    fn wanted_with_no_match_lists_what_was_found_and_the_fix() {
        // Given
        let candidates = [intel(), llvmpipe()];

        // When
        let error = choose(&candidates, Some("nvidia")).expect_err("no nvidia here");
        let message = error.to_string();

        // Then
        assert!(message.contains("contains \"nvidia\""));
        assert!(message.contains("Intel(R) UHD Graphics 630 (hardware, Vulkan)"));
        assert!(message.contains("llvmpipe (LLVM 19.1.7, 256 bits) (software, Vulkan)"));
        assert!(message.contains("Fix:"));
    }

    #[test]
    fn no_candidates_at_all_says_found_none() {
        // Given
        let candidates: [Candidate; 0] = [];

        // When
        let error = choose(&candidates, None).expect_err("nothing to choose");

        // Then
        assert!(error.to_string().contains("wanted any adapter; found none"));
    }
}
