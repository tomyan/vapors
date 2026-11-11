//! Scene ids held in slabs (IU3 3c-3d1): a kept box's texts keep their
//! ids across updates, a box painted again frees its ids for later
//! boxes to take, and a removed box's ids resolve no more.

mod support;

use std::panic::{AssertUnwindSafe, catch_unwind};

use support::{document, element, text, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Painted};
use vapors_scene::{Item, TextId};

/// `html` > `body` > four 20px divs, the first three with texts "a",
/// "b" and "c", the last empty; returns the divs and the texts.
fn rows(first: &str) -> (Document, Vec<NodeId>, Vec<NodeId>) {
    let mut doc = document();
    let css = format!("div {{ height: 20px; }} .first {{ {first} }}");
    doc.add_stylesheet("app.css", &css).expect("parses");
    let body = doc.body();
    let divs: Vec<NodeId> = ["first", "", "", ""]
        .into_iter()
        .map(|class| element(&mut doc, body, "div", class))
        .collect();
    let texts = ["a", "b", "c"]
        .into_iter()
        .zip(&divs)
        .map(|(run, &div)| text(&mut doc, div, run))
        .collect();
    (doc, divs, texts)
}

/// Each text item's id and the text it resolves to, in paint order.
fn text_items(painted: &Painted) -> Vec<(u32, String)> {
    let named = |item: &Item| match *item {
        Item::Text { text, .. } => Some((text.0, painted.text(text).text().to_owned())),
        _ => None,
    };
    painted
        .scene()
        .to_scene()
        .items()
        .iter()
        .filter_map(named)
        .collect()
}

/// Whether `id` names a text run in `painted`.
fn resolves(painted: &Painted, id: u32) -> bool {
    catch_unwind(AssertUnwindSafe(|| painted.text(TextId(id)).text().len())).is_ok()
}

#[test]
fn a_repainted_boxs_freed_id_is_taken_by_a_later_box_and_kept_ids_stay() {
    // Given: "a", "b" and "c" as texts 0, 1 and 2
    let (mut doc, divs, texts) = rows("");
    let first = text_items(doc.update(viewport(100.0, 100.0)).expect("updates"));
    assert_eq!(first, [(0, "a".into()), (1, "b".into()), (2, "c".into())]);

    // When: the first div loses its text and the last gains one
    doc.remove(texts[0]);
    text(&mut doc, divs[3], "d");
    let painted = doc.update(viewport(100.0, 100.0)).expect("updates");

    // Then: "b" and "c" keep their ids, "d" takes the freed 0, and no
    // other id resolves
    let now = text_items(painted);
    assert_eq!(now, [(1, "b".into()), (2, "c".into()), (0, "d".into())]);
    assert!(!resolves(painted, 3));
}

#[test]
fn a_removed_boxs_ids_are_freed() {
    // Given: the first div out of flow, so removing it moves no box,
    // and below the others, so it is still painted first
    let (mut doc, divs, _) = rows("position: absolute; top: 80px; z-index: -1;");
    let first = text_items(doc.update(viewport(100.0, 100.0)).expect("updates"));
    assert_eq!(first, [(0, "a".into()), (1, "b".into()), (2, "c".into())]);

    // When
    doc.remove(divs[0]);
    let painted = doc.update(viewport(100.0, 100.0)).expect("updates");

    // Then: "b" and "c", built again with the body, are painted again,
    // and of the three ids only the two they name resolve
    let now = text_items(painted);
    let texts: Vec<&str> = now.iter().map(|(_, text)| text.as_str()).collect();
    assert_eq!(texts, ["b", "c"]);
    let named: Vec<u32> = now.iter().map(|(id, _)| *id).collect();
    let mut resolving: Vec<u32> = (0..3).filter(|&id| resolves(painted, id)).collect();
    resolving.sort_by_key(|id| named.iter().position(|named| named == id));
    assert_eq!(resolving, named);
}
