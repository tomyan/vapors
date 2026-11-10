//! A stroke as bits: equal for strokes that rasterise alike, so the atlas
//! can key on them.

use zeno::Command;

use crate::icon::IconStroke;

pub(crate) fn key_bits(view_box: [f32; 4], stroke: &IconStroke) -> Vec<u32> {
    let mut bits: Vec<u32> = view_box.iter().map(|n| n.to_bits()).collect();
    bits.push(stroke.width.to_bits());
    for command in stroke.path.commands() {
        command_bits(command, &mut bits);
    }
    bits
}

/// A tag per command kind, then its points' coordinates.
fn command_bits(command: &Command, bits: &mut Vec<u32>) {
    let (tag, points) = match command {
        Command::MoveTo(p) => (0, vec![*p]),
        Command::LineTo(p) => (1, vec![*p]),
        Command::CurveTo(a, b, p) => (2, vec![*a, *b, *p]),
        Command::QuadTo(a, p) => (3, vec![*a, *p]),
        Command::Close => (4, vec![]),
    };
    bits.push(tag);
    bits.extend(points.iter().flat_map(|p| [p.x.to_bits(), p.y.to_bits()]));
}
