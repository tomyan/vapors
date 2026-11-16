use super::rowed_instances;

#[test]
#[should_panic(expected = "quads: an instance is 2 vec4s")]
fn an_instance_of_the_wrong_float_count_is_refused() {
    rowed_instances(("quads", 2), (&[()], &[]), |()| vec![0.0; 7]);
}
