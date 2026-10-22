//! Which device limits a window or a test device asks for: the defaults,
//! with the three that decide how large an image can be raised to what
//! the adapter reports. Everything else stays at the default so a device
//! behaves the same on every adapter in every other respect.

use std::fmt;

/// An adapter that offers less than the default for one of the raised
/// limits. `Display` names the limit and both numbers.
#[derive(Debug, PartialEq, Eq)]
pub struct LimitsBelowDefault {
    /// The limit's field name.
    pub limit: &'static str,
    /// What the adapter offers.
    pub offered: u64,
    /// The default it falls short of.
    pub default: u64,
}

impl fmt::Display for LimitsBelowDefault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the adapter offers {} = {}, below the default {}; this adapter cannot run vapors. \
             Fix: select another with VAPORS_GPU_ADAPTER or update the driver.",
            self.limit, self.offered, self.default
        )
    }
}

impl std::error::Error for LimitsBelowDefault {}

/// The default limits with `max_buffer_size`, `max_texture_dimension_2d`
/// and `max_storage_buffer_binding_size` raised to `adapter`'s.
///
/// # Errors
///
/// [`LimitsBelowDefault`] when the adapter offers less than the default
/// for any of the three, which no WebGPU-conformant adapter does.
pub fn raised_limits(adapter: &wgpu::Limits) -> Result<wgpu::Limits, LimitsBelowDefault> {
    let default = wgpu::Limits::default();
    let raised = [
        (
            "max_buffer_size",
            adapter.max_buffer_size,
            default.max_buffer_size,
        ),
        (
            "max_texture_dimension_2d",
            u64::from(adapter.max_texture_dimension_2d),
            u64::from(default.max_texture_dimension_2d),
        ),
        (
            "max_storage_buffer_binding_size",
            adapter.max_storage_buffer_binding_size,
            default.max_storage_buffer_binding_size,
        ),
    ];
    for (limit, offered, default) in raised {
        if offered < default {
            return Err(LimitsBelowDefault {
                limit,
                offered,
                default,
            });
        }
    }
    Ok(wgpu::Limits {
        max_buffer_size: adapter.max_buffer_size,
        max_texture_dimension_2d: adapter.max_texture_dimension_2d,
        max_storage_buffer_binding_size: adapter.max_storage_buffer_binding_size,
        ..default
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generous() -> wgpu::Limits {
        wgpu::Limits {
            max_buffer_size: 1 << 31,
            max_texture_dimension_2d: 16384,
            max_storage_buffer_binding_size: 1 << 30,
            max_bind_groups: 8,
            ..wgpu::Limits::default()
        }
    }

    #[test]
    fn the_three_image_limits_are_raised_and_nothing_else_is() {
        // When
        let limits = raised_limits(&generous()).expect("above default");

        // Then
        assert_eq!(limits.max_buffer_size, 1 << 31);
        assert_eq!(limits.max_texture_dimension_2d, 16384);
        assert_eq!(limits.max_storage_buffer_binding_size, 1 << 30);
        assert_eq!(
            limits.max_bind_groups,
            wgpu::Limits::default().max_bind_groups
        );
    }

    #[test]
    fn an_adapter_below_the_default_is_refused_by_limit_name() {
        // Given
        let adapter = wgpu::Limits {
            max_texture_dimension_2d: 4096,
            ..generous()
        };

        // When
        let error = raised_limits(&adapter).expect_err("below default");

        // Then
        assert_eq!(error.limit, "max_texture_dimension_2d");
        assert_eq!(error.offered, 4096);
        assert!(
            error.to_string().contains("below the default 8192"),
            "{error}"
        );
    }
}
