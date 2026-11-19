//! The HUD laid out at 320 px wide through vapors' own layout, in a
//! portrait phone (568 high) and a short landscape one (400 high, the
//! `short` state): the speed readout stays on one line, and neither
//! the controls, the stats nor the note overlap. The same layouts at
//! 360 px pin the unchanged rules, and one test the 321 px edge of the query.
//! Reproduce: `CARGO_INCREMENTAL=0 cargo test -p vapors-racer --lib hud_320`.
use super::app::Racer;
use vapors_dom::NodeId;
use vapors_elements::{BoxRect, Viewport};

/// A racer laid out at `width` x `height` by the app's own `sync_hud`,
/// its speed and FPS at three and two digits, the paused note shown.
struct Scene {
    racer: Racer,
}

impl Scene {
    fn new(width: f32, height: f32) -> Self {
        let racer = Racer::new();
        let viewport = Viewport {
            width,
            height,
            scale: 1.0,
        };
        racer.sync_hud(viewport);
        racer.hud.speed.set("128".into());
        racer.hud.fps.set("60".into());
        racer.document.borrow_mut().update(viewport).unwrap();
        Self { racer }
    }

    /// Every element `pick` accepts, given its attribute lookup.
    fn find(&self, pick: impl Fn(&dyn Fn(&str) -> Option<String>) -> bool) -> Vec<NodeId> {
        let doc = self.racer.document.borrow();
        let all = doc.elements();
        let doc = &*doc;
        let wanted = |&n: &NodeId| pick(&|a: &str| doc.attribute(n, a).map(str::to_string));
        all.into_iter().filter(wanted).collect()
    }

    /// The one element of `class`; refuses none or several.
    fn one(&self, class: &str) -> NodeId {
        let found = self.find(|get| get("class").as_deref() == Some(class));
        assert_eq!(found.len(), 1, "expected one .{class}, found {found:?}");
        found[0]
    }

    /// The controls: actions and holds.
    fn controls(&self) -> Vec<NodeId> {
        self.find(|get| get("data-action").is_some() || get("data-hold").is_some())
    }

    /// `node`'s border box; refuses an element with none.
    fn rect(&self, node: NodeId) -> BoxRect {
        let doc = self.racer.document.borrow();
        let painted = doc.painted().unwrap();
        painted
            .border_box(node)
            .unwrap_or_else(|| panic!("{node:?} has no border box; is it shown?"))
    }

    fn children(&self, node: NodeId) -> Vec<NodeId> {
        self.racer.document.borrow().children(node)
    }
}

fn intersects(a: &BoxRect, b: &BoxRect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

/// 320 is the narrowest phone; the others are wider than the media rules reach
/// (a short 360 wide already wraps KM/H on origin/main, left as it was).
const SIZES: [(f32, f32); 4] = [
    (320.0, 568.0),
    (320.0, 400.0),
    (360.0, 568.0),
    (640.0, 400.0),
];

#[test]
fn hud_320_speed_value_and_caption_are_one_line() {
    for (width, height) in SIZES {
        let scene = Scene::new(width, height);
        let stats = scene.children(scene.one("stats"));
        assert_eq!(stats.len(), 4, "KM/H, LAP, TIME and FPS");
        let parts = |stat: NodeId| {
            let kids = scene.children(stat);
            assert_eq!(kids.len(), 2, "a caption and a value");
            (scene.rect(kids[0]), scene.rect(kids[1]), scene.rect(stat))
        };
        let (caption, value, stat) = parts(stats[0]);
        let (fps_caption, fps_value, fps_stat) = parts(stats[3]);
        let at = format!("at {width} x {height}");
        assert_eq!(caption.height, fps_caption.height, "{at}: KM/H wraps");
        assert_eq!(value.height, fps_value.height, "{at}: 128 wraps");
        assert_eq!(stat.height, fps_stat.height, "{at}: the speed stat wraps");
        if height < 480.0 {
            let row = caption.y < value.y + value.height && value.y < caption.y + caption.height;
            assert!(row, "{at}: KM/H {caption:?} is not beside 128 {value:?}");
        }
    }
}

#[test]
fn hud_320_stats_note_and_controls_are_inside_and_apart() {
    for (width, height) in SIZES {
        let scene = Scene::new(width, height);
        let mut named: Vec<(String, BoxRect)> = vec![
            ("stats".into(), scene.rect(scene.one("stats"))),
            ("note".into(), scene.rect(scene.one("note"))),
        ];
        let controls = scene.controls();
        assert_eq!(controls.len(), 8, "tune, restart, toggle, camera, 4 pedals");
        named.extend(controls.iter().map(|&c| (format!("{c:?}"), scene.rect(c))));
        for (index, (name, a)) in named.iter().enumerate() {
            let at = format!("at {width} x {height}");
            assert!(
                a.x >= 0.0 && a.x + a.width <= width,
                "{at}: {name} {a:?} leaves it"
            );
            for (other, b) in &named[index + 1..] {
                assert!(
                    !intersects(a, b),
                    "{at}: {name} {a:?} overlaps {other} {b:?}"
                );
            }
        }
    }
}

/// The stats box at 321 x 400, as `origin/main` laid it out: the
/// `(width <= 320px)` rules must not reach it.
#[test]
fn hud_320_media_rules_stop_at_320() {
    let scene = Scene::new(321.0, 400.0);
    let stats = scene.rect(scene.one("stats"));
    assert!(
        (stats.width - 336.71).abs() < 0.01,
        "at 321 x 400 the stats are {stats:?}, not 336.71 wide as before"
    );
}
