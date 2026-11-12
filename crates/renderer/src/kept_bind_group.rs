//! A pipeline's bind group kept between frames while the uniform
//! buffer it binds is the one the pipeline writes.

/// The bind group made for one generation of a [`UniformSlots`](crate::UniformSlots)
/// buffer, or none yet.
pub struct KeptBindGroup {
    made: Option<(u64, wgpu::BindGroup)>,
}

impl KeptBindGroup {
    /// No bind group yet: made on the first draw.
    pub const NONE: Self = Self { made: None };

    /// The bind group for the buffer of `generation`: the kept one, or
    /// one `make` makes (counting it) when the kept one is for another
    /// buffer or there is none.
    pub fn of(
        &mut self,
        generation: u64,
        make: impl FnOnce() -> wgpu::BindGroup,
    ) -> &wgpu::BindGroup {
        let stale = self
            .made
            .as_ref()
            .is_none_or(|(made_for, _)| *made_for != generation);
        if stale {
            self.made = None;
        }
        &self.made.get_or_insert_with(|| (generation, make())).1
    }

    /// How many bind groups are kept: 0 or 1.
    #[must_use]
    pub fn kept(&self) -> u64 {
        u64::from(self.made.is_some())
    }
}
