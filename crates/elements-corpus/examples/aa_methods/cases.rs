//! What S3b measures (docs/plans/one-scene.md): touche's ten icons
//! (`fixtures/icons_paint.wisp`), each in a 48 px cell, under four
//! placements, beside a sub-pixel stroke and a translucent overlap.

/// One icon: its name and its path, stroked 1.5 wide in a 16 unit
/// viewBox, as touche's top bar has them.
pub struct Icon {
    pub name: &'static str,
    pub path: &'static str,
    /// The stroke width; 0 fills the path instead.
    pub stroke: f32,
}

/// A closed form beside the path that draws it, for the closed-form
/// kind: its geometry for the closed-form pipeline, the path for
/// Chromium and the mesh methods.
pub struct ClosedCase {
    pub shape: Icon,
    pub geometry: vapors_shapes::Geometry,
}

pub fn closed_cases() -> [ClosedCase; 4] {
    use vapors_shapes::Geometry::{Ellipse, Rect};
    let circle = Ellipse {
        centre: [8.0, 8.0],
        radii: [6.0, 6.0],
    };
    let case = |name, path, stroke, geometry| ClosedCase {
        shape: Icon { name, path, stroke },
        geometry,
    };
    let round = "M5 4h6a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2h-6a2 2 0 0 1-2-2v-4a2 2 0 0 1 2-2z";
    [
        case(
            "circle fill",
            "M2 8a6 6 0 1 0 12 0a6 6 0 1 0-12 0z",
            0.0,
            circle,
        ),
        case(
            "circle stroke",
            "M2 8a6 6 0 1 0 12 0a6 6 0 1 0-12 0z",
            1.5,
            Ellipse {
                centre: [8.0, 8.0],
                radii: [6.0, 6.0],
            },
        ),
        case(
            "rect fill",
            "M3 4h10v8h-10z",
            0.0,
            Rect {
                origin: [3.0, 4.0],
                size: [10.0, 8.0],
                radius: [0.0, 0.0],
            },
        ),
        case(
            "round rect stroke",
            round,
            1.5,
            Rect {
                origin: [3.0, 4.0],
                size: [10.0, 8.0],
                radius: [2.0, 2.0],
            },
        ),
    ]
}

/// The closed forms at a quarter-pixel stroke (#345 review, M4): a
/// circle and a rect, for the closed-form pipeline's ramp.
pub fn sub_pixel_closed_cases() -> [ClosedCase; 2] {
    use vapors_shapes::Geometry::{Ellipse, Rect};
    let case = |name, path, geometry| ClosedCase {
        shape: Icon {
            name,
            path,
            stroke: 0.25,
        },
        geometry,
    };
    [
        case(
            "0.25px circle",
            "M2 8a6 6 0 1 0 12 0a6 6 0 1 0-12 0z",
            Ellipse {
                centre: [8.0, 8.0],
                radii: [6.0, 6.0],
            },
        ),
        case(
            "0.25px rect",
            "M3 4h10v8h-10z",
            Rect {
                origin: [3.0, 4.0],
                size: [10.0, 8.0],
                radius: [0.0, 0.0],
            },
        ),
    ]
}

pub const ICONS: [Icon; 10] = [
    icon("open", "M1.5 4.5v8h13v-7h-6l-1.5-1.5h-5.5z"),
    icon("paste", "M5 3H3v11h10V3h-2M5.5 2h5v2.5h-5z"),
    icon("export", "M8 10V2M5 5l3-3 3 3M3 9v5h10V9"),
    icon("undo", "M5 3L2 6l3 3M2 6h7a4 4 0 0 1 0 8H6"),
    icon("redo", "M11 3l3 3-3 3M14 6H7a4 4 0 0 0 0 8h3"),
    icon("tools", "M2 4h12M2 8h12M2 12h12M5 2v4M11 6v4M7 10v4"),
    icon("expand", "M3 10l5-5 5 5"),
    icon("voice", "M2 6v4M5 4v8M8 2v12M11 4v8M14 6v4"),
    icon("mute", "M2 6h3l4-3v10l-4-3H2zM11 6l3 4M14 6l-3 4"),
    icon("send", "M2 8h11M9 4l4 4-4 4"),
];

const fn icon(name: &'static str, path: &'static str) -> Icon {
    Icon {
        name,
        path,
        stroke: 1.5,
    }
}

/// The extra cases for the known limits: a stroke a quarter pixel wide,
/// and Tools in translucent paint where its lines cross.
pub const SUB_PIXEL: Icon = Icon {
    name: "0.25px line",
    path: "M2 8.5h12M2 3.2l12 9",
    stroke: 0.25,
};

/// A placement: CSS's transform on the 16 px svg about its centre, and
/// the device pixels per CSS pixel.
#[derive(Clone, Copy)]
pub struct Placement {
    pub name: &'static str,
    pub css: &'static str,
    /// The CSS transform's linear part, [a, b, c, d].
    pub linear: [f32; 4],
    pub scale: u32,
}

pub fn placements() -> [Placement; 4] {
    let (sin, cos) = 30f32.to_radians().sin_cos();
    [
        Placement {
            name: "1x",
            css: "none",
            linear: [1.0, 0.0, 0.0, 1.0],
            scale: 1,
        },
        Placement {
            name: "2x",
            css: "none",
            linear: [1.0, 0.0, 0.0, 1.0],
            scale: 2,
        },
        Placement {
            name: "rotated 30",
            css: "rotate(30deg)",
            linear: [cos, sin, -sin, cos],
            scale: 1,
        },
        Placement {
            name: "scaled 1.37",
            css: "scale(1.37)",
            linear: [1.37, 0.0, 0.0, 1.37],
            scale: 1,
        },
    ]
}

/// A cell's side and where the svg sits in it, CSS pixels.
pub const CELL: u32 = 48;
pub const AT: f32 = 16.0;

/// The paint: dark strokes on a light page, as the icons' first row.
pub const PAGE: [u8; 4] = [0xec, 0xec, 0xf0, 0xff];
pub const INK: [u8; 4] = [0x1d, 0x1d, 0x1f, 0xff];
/// The translucent row's ink: alpha 0.5.
pub const FAINT: [u8; 4] = [0x1d, 0x1d, 0x1f, 0x80];

/// The transform from the icon's viewBox to the cell's device pixels
/// for cell `index` under `placement`: about the svg's centre.
pub fn transform(placement: &Placement, index: u32) -> [f32; 6] {
    let s = placement.scale as f32;
    let [a, b, c, d] = placement.linear.map(|v| v * s);
    let centre = [(index * CELL) as f32 + AT + 8.0, AT + 8.0].map(|v| v * s);
    [
        a,
        b,
        c,
        d,
        centre[0] - (a * 8.0 + c * 8.0),
        centre[1] - (b * 8.0 + d * 8.0),
    ]
}
