//! The lights page (`pages/lights_page.wisp`) on a [`Stage`].

use super::lights_page::{LightsPage, LightsPageProps};
use super::{Stage, at};
use std::cell::RefCell;
use std::rc::Rc;
use vapors_scene3d::{Ambient, Colour, Light, Scene};
use vapors_signals::Signal;

pub struct LightsRig {
    pub scene: Rc<RefCell<Scene>>,
    pub place: Signal<vapors_scene3d::Transform>,
    pub page: LightsPage,
    _stage: Stage,
}

pub fn white() -> Colour {
    Colour::srgb(1.0, 1.0, 1.0).unwrap()
}

impl LightsRig {
    /// A sun, `lamps` point lights sharing one transform, and `ambient`.
    pub fn new(lamps: u32, ambient: Ambient) -> Self {
        let stage = Stage::new();
        let place = Signal::new(at(0.0));
        let colour = white();
        let props = LightsPageProps {
            scene: Some(Rc::clone(&stage.scene)),
            surface: Some(stage.surface.clone()),
            sun: Some(Light::Directional {
                colour,
                intensity: 1.0,
            }),
            lamp: Some(Light::Point {
                colour,
                intensity: 2.0,
                range: 5.0,
            }),
            lamps: Signal::new(lamps),
            place,
            ambient,
        };
        let body = stage.doc.borrow().body();
        let page = LightsPage::mount(&stage.doc, body, props);
        Self {
            scene: Rc::clone(&stage.scene),
            place,
            page,
            _stage: stage,
        }
    }
}
