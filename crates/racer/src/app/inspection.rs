//! Read-only QA telemetry, requested explicitly by ?inspect=1.
use super::Racer;
impl Racer {
    pub(super) fn report_frame(&self, frame: u64) {
        if !self.inspect {
            if frame == 0 {
                vapors_platform::report_status("racer ready");
            }
            return;
        }
        let controls = self.driving.controls();
        let document = self.document.borrow();
        let targets: Vec<String> = document
            .elements()
            .iter()
            .filter_map(|node| {
                let name = document
                    .attribute(*node, "data-action")
                    .or_else(|| document.attribute(*node, "data-drive"))
                    .or_else(|| document.attribute(*node, "data-setting"))?;
                let rect = document.painted()?.border_box(*node)?;
                Some(format!(
                    "\"{name}\":[{},{},{},{}]",
                    rect.x, rect.y, rect.width, rect.height
                ))
            })
            .collect();
        let camera = format!(
            "\"cameraFollow\":{},\"cameraYaw\":{},\"cameraTilt\":{},\"cameraMoving\":{}",
            self.camera.follow, self.camera.rotation, self.camera.tilt, self.camera.moving
        );
        vapors_platform::report_status(&format!(
            "{{\"frame\":{frame},\"paused\":{},\"speed\":{},\"zoom\":{},\"time\":{},\"x\":{},\"y\":{},\"heading\":{},\"laps\":{},\"throttle\":{},\"steer\":{},\"steering\":{},\"grip\":{},{camera},\"targets\":{{{}}}}}",
            self.paused,
            self.race.speed,
            self.camera.zoom,
            self.race.elapsed,
            self.race.position[0],
            self.race.position[1],
            self.race.heading,
            self.race.progress.laps,
            controls.throttle,
            controls.steer,
            self.hud.steering.get(),
            self.hud.grip.get(),
            targets.join(",")
        ));
    }
}
