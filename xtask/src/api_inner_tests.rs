use std::io::{Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;

use super::copied;

#[test]
fn bytes_are_copied_to_the_end_and_the_far_side_sees_the_end() {
    // Given: a source that sends and closes, and a destination pair
    let (mut source, from) = UnixStream::pair().expect("a pair");
    let (to, mut receiver) = UnixStream::pair().expect("a pair");
    source.write_all(b"POST /v1/messages").expect("sent");
    source.shutdown(Shutdown::Write).expect("shut");

    // When
    copied(from, to, |out| out.shutdown(Shutdown::Write));

    // Then: the bytes arrive, then the end of the stream
    let mut text = String::new();
    receiver.read_to_string(&mut text).expect("read to the end");
    assert_eq!(text, "POST /v1/messages");
}
