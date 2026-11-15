use crate::kept_order::KeptOrder;

#[test]
fn a_label_of_a_kept_step_gives_its_position() {
    let order = KeptOrder {
        labels: vec![10, 20, 30],
        ..KeptOrder::default()
    };
    assert_eq!(order.position_of(20), 1);
}

#[test]
#[should_panic(expected = "label 15 names no step of the kept paint order")]
fn a_label_of_no_kept_step_is_refused_naming_it() {
    let order = KeptOrder {
        labels: vec![10, 20, 30],
        ..KeptOrder::default()
    };
    order.position_of(15);
}
