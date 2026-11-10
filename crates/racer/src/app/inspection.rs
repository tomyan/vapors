//! Read-only QA telemetry, requested explicitly by ?inspect=1.
use super::Racer;
use std::fmt::Display;

/// The status object's fields as JSON text, refusing a number JSON
/// cannot carry.
#[derive(Default)]
pub(crate) struct Fields(Vec<String>);

impl Fields {
    pub(crate) fn number<T>(&mut self, name: &str, value: T) -> Result<(), String>
    where
        T: Display + Into<f64> + Copy,
    {
        if !value.into().is_finite() {
            return Err(format!(
                "?inspect=1 status field `{name}` is {value}, which JSON cannot carry; find what made it non-finite"
            ));
        }
        self.plain(name, value);
        Ok(())
    }
    fn plain(&mut self, name: &str, value: impl Display) {
        self.0.push(format!("\"{name}\":{value}"));
    }
    pub(crate) fn object(&self) -> String {
        format!("{{{}}}", self.0.join(","))
    }
}

impl Racer {
    pub(super) fn report_frame(&self, frame: u64) -> Result<(), String> {
        if self.inspect {
            vapors_platform::report_status(&self.status(frame)?);
        } else if frame == 0 {
            vapors_platform::report_status("racer ready");
        }
        Ok(())
    }
    pub(crate) fn status(&self, frame: u64) -> Result<String, String> {
        let mut fields = Fields::default();
        fields.plain("frame", frame);
        fields.plain("paused", self.paused);
        self.race_fields(&mut fields)?;
        self.camera_fields(&mut fields)?;
        fields.plain("targets", self.targets()?.object());
        Ok(fields.object())
    }
    fn race_fields(&self, fields: &mut Fields) -> Result<(), String> {
        let controls = self.driving.controls();
        let race = &self.race;
        fields.number("speed", race.speed)?;
        fields.number("time", race.elapsed)?;
        fields.number("x", race.position[0])?;
        fields.number("y", race.position[1])?;
        fields.number("heading", race.heading)?;
        fields.plain("laps", race.progress.laps);
        fields.number("throttle", controls.throttle)?;
        fields.number("steer", controls.steer)?;
        fields.number("steering", self.hud.steering.get())?;
        fields.number("grip", self.hud.grip.get())
    }
    fn camera_fields(&self, fields: &mut Fields) -> Result<(), String> {
        fields.number("zoom", self.camera.zoom)?;
        fields.plain("cameraFollow", self.camera.follow);
        fields.number("cameraYaw", self.camera.rotation)?;
        fields.number("cameraTilt", self.camera.tilt)?;
        fields.plain("cameraMoving", self.camera.moving);
        Ok(())
    }
    fn targets(&self) -> Result<Fields, String> {
        let document = self.document.borrow();
        let mut targets = Fields::default();
        for node in document.elements() {
            let name = ["data-action", "data-hold", "data-setting"]
                .iter()
                .find_map(|attribute| document.attribute(node, attribute));
            let rect = document.painted().and_then(|p| p.border_box(node));
            if let (Some(name), Some(rect)) = (name, rect) {
                targets.plain(
                    name,
                    rect_json(name, [rect.x, rect.y, rect.width, rect.height])?,
                );
            }
        }
        Ok(targets)
    }
}

pub(super) fn rect_json(name: &str, numbers: [f32; 4]) -> Result<String, String> {
    let mut parts = Fields::default();
    for (edge, value) in ["x", "y", "width", "height"].iter().zip(numbers) {
        parts.number(&format!("targets.{name}.{edge}"), value)?;
    }
    let values: Vec<String> = numbers.iter().map(ToString::to_string).collect();
    Ok(format!("[{}]", values.join(",")))
}
