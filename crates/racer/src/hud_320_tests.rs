//! The HUD laid out at 320 px wide through vapors' own layout, in a
//! portrait phone and in a short landscape one (the `short` state):
//! the speed readout stays on one line and no two controls overlap.
//! Reproduce: `CARGO_INCREMENTAL=0 cargo test -p vapors-racer --lib hud_320`.
use super::app::Racer;
use vapors_elements::{BoxRect, Document, Viewport};

/// A racer's document laid out at 320 px wide and `height`, its speed
/// at three digits as at speed, as the app sets `short` and `compact`.
fn laid_out(height: f32) -> Racer {
    let racer = Racer::new();
    racer.hud.compact.set(true);
    racer.hud.short.set(height < 480.0);
    racer.hud.speed.set("128".into());
    let viewport = Viewport {
        width: 320.0,
        height,
        scale: 1.0,
    };
    racer.document.borrow_mut().update(viewport).unwrap();
    racer
}

/// The border boxes of the elements `keep` accepts (given an attribute
/// lookup), each named by its action, hold or class.
fn boxes(
    doc: &Document,
    keep: impl Fn(&dyn Fn(&str) -> Option<String>) -> bool,
) -> Vec<(String, BoxRect)> {
    let painted = doc.painted().unwrap();
    let mut found = Vec::new();
    for node in doc.elements() {
        let get = |name: &str| doc.attribute(node, name).map(str::to_string);
        let Some(rect) = painted.border_box(node) else {
            continue;
        };
        if keep(&get) {
            let name = ["data-action", "data-hold", "class"]
                .iter()
                .find_map(|a| get(a));
            found.push((name.unwrap_or_default(), rect));
        }
    }
    found
}

fn intersects(a: &BoxRect, b: &BoxRect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

#[test]
fn hud_320_controls_do_not_overlap() {
    for height in [568.0, 400.0] {
        let racer = laid_out(height);
        let doc = racer.document.borrow();
        let controls = boxes(&doc, |get| {
            get("data-action").is_some() || get("data-hold").is_some()
        });
        assert_eq!(
            controls.len(),
            8,
            "tune, restart, toggle, camera and four pedals"
        );
        for (index, (name, a)) in controls.iter().enumerate() {
            for (other, b) in &controls[index + 1..] {
                assert!(
                    !intersects(a, b),
                    "at 320 x {height}: {name} {a:?} overlaps {other} {b:?}"
                );
            }
        }
    }
}

#[test]
fn hud_320_speed_readout_is_one_line() {
    for height in [568.0, 400.0] {
        let racer = laid_out(height);
        let doc = racer.document.borrow();
        let captions = boxes(&doc, |get| get("class").as_deref() == Some("caption"));
        let stats: Vec<_> = captions.iter().filter(|(_, r)| r.y < 80.0).collect();
        assert_eq!(stats.len(), 4, "KM/H, LAP, TIME and FPS captions");
        let one_line = stats[1].1.height;
        assert_eq!(
            stats[0].1.height, one_line,
            "at 320 x {height}: KM/H wraps: {:?}",
            stats[0].1
        );
        let stats_box = boxes(&doc, |get| get("class").as_deref() == Some("stats"));
        assert!(
            stats_box[0].1.x + stats_box[0].1.width <= 320.0,
            "stats overflow: {stats_box:?}"
        );
    }
}
