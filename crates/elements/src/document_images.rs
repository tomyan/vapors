//! The images a document's `img` elements show (`images.rs`), registered
//! and dropped by name, and the device limit their sides are checked
//! against.

use crate::document::Document;

impl Document {
    /// Registers `image` under `name` for the `img` elements whose `src`
    /// is `name`, replacing any image of that name; they show it from
    /// the next update.
    ///
    /// # Errors
    /// An image that cannot be drawn, naming it: a side of zero, a side
    /// over the device's limit (8192 px until
    /// [`set_max_image_side`](Self::set_max_image_side) is called), or
    /// pixels that do not match its size.
    pub fn register_image(
        &mut self,
        name: &str,
        image: vapors_scene::RgbaImage,
    ) -> Result<(), String> {
        self.images.register(name, image)?;
        self.note_change();
        self.boxes.marks.all = true;
        Ok(())
    }

    /// Drops the image registered as `name`, so its pixels are freed;
    /// the painter drops its texture after the next frame that does not
    /// draw it.
    ///
    /// # Errors
    /// No image registered as `name`, or an `img` that still names it.
    pub fn unregister_image(&mut self, name: &str) -> Result<(), String> {
        let nodes = crate::check::elements(&self.arena, self.root());
        self.images.unregister(name, &self.arena, &nodes)
    }

    /// The longest side an image may have on the device this document
    /// is drawn on: its `max_texture_dimension_2d`. `ElementsApp` sets
    /// it from each frame's device. Until it is set, a side over 8192 px
    /// (WebGPU's guaranteed minimum) is refused, since the device that
    /// will upload it is not known.
    pub fn set_max_image_side(&mut self, side: u32) {
        self.images.set_max_side(side);
    }
}
