use std::time::Duration;

use winit::event_loop::ControlFlow;

use super::Next;
use crate::Instant;

#[test]
fn each_next_waits_for_events_alone_except_wait_until_which_waits_until_its_time() {
    let at = Instant::now() + Duration::from_millis(1000);
    let cases = [
        (Next::Continue, ControlFlow::Wait),
        (Next::Wait, ControlFlow::Wait),
        (Next::WaitUntil(at), ControlFlow::WaitUntil(at)),
        (Next::Exit, ControlFlow::Wait),
    ];
    for (next, flow) in cases {
        assert_eq!(next.control_flow(), flow, "{next:?}");
    }
}
