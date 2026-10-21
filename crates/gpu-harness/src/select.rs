//! Choosing one adapter from the ones the instance found.

use crate::ADAPTER_ENV;

/// Pick the adapter, or panic with the list of candidates and the fix.
pub fn adapter(candidates: Vec<wgpu::Adapter>, wanted: Option<&str>) -> wgpu::Adapter {
    let names: Vec<String> = candidates.iter().map(describe).collect();
    let chosen = match wanted {
        Some(text) => candidates.into_iter().find(|a| {
            a.get_info()
                .name
                .to_lowercase()
                .contains(&text.to_lowercase())
        }),
        None => first_hardware_else_any(candidates),
    };
    chosen.unwrap_or_else(|| panic!("{}", no_adapter_message(&names, wanted)))
}

fn first_hardware_else_any(candidates: Vec<wgpu::Adapter>) -> Option<wgpu::Adapter> {
    let hardware = candidates
        .iter()
        .position(|a| a.get_info().device_type != wgpu::DeviceType::Cpu);
    let index = hardware.or(if candidates.is_empty() { None } else { Some(0) })?;
    candidates.into_iter().nth(index)
}

fn describe(adapter: &wgpu::Adapter) -> String {
    let info = adapter.get_info();
    format!("{} ({:?}, {:?})", info.name, info.device_type, info.backend)
}

/// The failure text: what was wanted, what was found, what to do.
pub fn no_adapter_message(found: &[String], wanted: Option<&str>) -> String {
    let wanted = match wanted {
        Some(text) => format!("an adapter whose name contains \"{text}\" ({ADAPTER_ENV})"),
        None => "any adapter".to_owned(),
    };
    let found = if found.is_empty() {
        "none".to_owned()
    } else {
        found.join("; ")
    };
    format!(
        "no GPU adapter: wanted {wanted}; found {found}. \
         Fix: install a Vulkan driver (mesa-vulkan-drivers gives lavapipe on Linux), \
         check VK_ICD_FILENAMES if set, or set {ADAPTER_ENV} to a name in the found list."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_names_the_wanted_adapter_and_the_found_ones() {
        // Given
        let found = vec!["llvmpipe (Cpu, Vulkan)".to_owned()];

        // When
        let message = no_adapter_message(&found, Some("intel"));

        // Then
        assert!(message.contains("contains \"intel\""));
        assert!(message.contains("found llvmpipe (Cpu, Vulkan)"));
        assert!(message.contains("Fix:"));
    }

    #[test]
    fn message_says_none_when_nothing_was_found() {
        // Given
        let found: Vec<String> = Vec::new();

        // When
        let message = no_adapter_message(&found, None);

        // Then
        assert!(message.contains("wanted any adapter; found none"));
    }
}
