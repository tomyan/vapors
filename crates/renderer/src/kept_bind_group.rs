//! A pipeline's bind group kept between frames while the uniform
//! buffer it binds is the one the pipeline writes.

/// The generations of the two buffers a bind group binds: a
/// [`UniformSlots`](crate::UniformSlots) buffer's and a
/// [`TransformTable`](crate::TransformTable)'s (0 for a bind group over
/// the slots alone).
pub type Generations = (u64, u64);

/// The bind group made for one set of buffer generations `G`, or none
/// yet: a pair ([`Generations`]) by default, three for the instanced
/// pipelines' viewports, transform table and clip table (`instanced.rs`).
pub struct KeptBindGroup<G = Generations> {
    made: Option<(G, wgpu::BindGroup)>,
}

impl<G: PartialEq + Copy> KeptBindGroup<G> {
    /// No bind group yet: made on the first draw.
    pub const NONE: Self = Self { made: None };

    /// The bind group for the buffers of `generation`: the kept one, or
    /// one `make` makes (counting it) when the kept one is for other
    /// buffers or there is none.
    pub fn of(
        &mut self,
        generation: G,
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
