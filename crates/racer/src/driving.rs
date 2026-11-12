//! Held keyboard and independent touch/pointer controls.
use crate::model::Controls;
use std::collections::BTreeMap;
#[derive(Default)]
pub(crate) struct Driving {
    held: BTreeMap<String, String>,
}
impl Driving {
    pub(crate) fn key(&mut self, code: &str, pressed: bool) {
        let control = match code {
            "KeyW" | "ArrowUp" => "gas",
            "KeyS" | "ArrowDown" => "brake",
            "KeyA" | "ArrowLeft" => "left",
            "KeyD" | "ArrowRight" => "right",
            _ => return,
        };
        self.hold(
            code.into(),
            if pressed { Some(control.into()) } else { None },
        );
    }
    pub(crate) fn hold(&mut self, id: String, control: Option<String>) {
        if let Some(control) = control {
            self.held.insert(id, control);
        } else {
            self.held.remove(&id);
        }
    }
    pub(crate) fn clear(&mut self) {
        self.held.clear();
    }
    pub(crate) fn controls(&self) -> Controls {
        let held = |name: &str| {
            if self.held.values().any(|value| value == name) {
                1.0
            } else {
                0.0
            }
        };
        Controls {
            throttle: held("gas"),
            brake: held("brake"),
            steer: held("right") - held("left"),
        }
    }
}
