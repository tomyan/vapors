//! Helpers that were in `tests/animation_sample.rs`.
#![allow(dead_code)]

use super::{chain, env, sheet};
use vapors_css::animation::{Animation, animations};
use vapors_css::cascade::Cascade;
use vapors_css::value::{Color, ColorScheme, Length, Value};
use vapors_scene::Color as Rgba;

pub fn px(v: f64) -> Value {
    Value::Length(Length::px(v))
}

pub fn rgb(r: u8, g: u8, b: u8) -> Value {
    Value::Color(Color::Rgba(Rgba::from_rgba8(r, g, b, 255)))
}

pub fn pulse_animation(src: &str, scheme: ColorScheme) -> Animation {
    let (tree, leaf) = chain(&["div.pulse"]);
    let cascade = Cascade::new().author(sheet(src));
    let mut e = env();
    e.color_scheme = scheme;
    let styles = cascade
        .compute_tree(&tree.node(tree.root()), &e)
        .expect("computes");
    let style = &styles.iter().find(|(n, _)| n.id() == leaf).expect("leaf").1;
    let spec = animations(style).into_iter().next().expect("one animation");
    cascade
        .animation(&spec, style, &e)
        .unwrap_or_else(|err| panic!("{err}"))
}
