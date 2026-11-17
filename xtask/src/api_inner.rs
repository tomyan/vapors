//! `xtask api-relay-inner`: the session's parent inside the sandbox
//! (`docs/plans/api-builder.md`). It listens on loopback, where the
//! session's `ANTHROPIC_BASE_URL` points, relays each connection to the
//! harness socket outside, the sandbox's one way out, and runs the
//! command it is given, exiting with its status. It is the trusted
//! binary, bound read-only; the session cannot change it.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::thread;

/// Relays loopback `port` to `socket` and runs `command`.
pub fn run(port: u16, socket: &Path, command: &[String]) -> ExitCode {
    let Some((program, args)) = command.split_first() else {
        eprintln!(
            "FAIL api-relay-inner: no command to run; usage: xtask api-relay-inner --port P --socket S -- <command>"
        );
        return ExitCode::FAILURE;
    };
    let listener = match TcpListener::bind(("127.0.0.1", port)) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("FAIL api-relay-inner: cannot listen on 127.0.0.1:{port}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let socket = socket.to_owned();
    thread::spawn(move || relay_each(&listener, &socket));
    exit_of(Command::new(program).args(args).status())
}

fn exit_of(status: std::io::Result<std::process::ExitStatus>) -> ExitCode {
    match status {
        Ok(status) => ExitCode::from(
            status
                .code()
                .and_then(|code| u8::try_from(code).ok())
                .unwrap_or(1),
        ),
        Err(error) => {
            eprintln!("FAIL api-relay-inner: the command did not start: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Each connection to `listener` joined to a new one to `socket`. A
/// connection has no deadline of its own: a streamed response lasts as
/// long as the model writes, and the run's deadline (outside) ends the
/// sandbox and every connection with it.
fn relay_each(listener: &TcpListener, socket: &PathBuf) {
    for stream in listener.incoming().flatten() {
        match UnixStream::connect(socket) {
            Ok(out) => joined(stream, out),
            Err(error) => {
                eprintln!(
                    "api-relay-inner: the harness socket {} refused: {error}",
                    socket.display()
                );
                let _ = stream.shutdown(Shutdown::Both);
            }
        }
    }
}

fn joined(inside: TcpStream, outside: UnixStream) {
    let (Ok(inside_read), Ok(outside_read)) = (inside.try_clone(), outside.try_clone()) else {
        eprintln!("api-relay-inner: a connection could not be split; dropped");
        return;
    };
    thread::spawn(move || copied(inside_read, outside, |out| out.shutdown(Shutdown::Write)));
    thread::spawn(move || copied(outside_read, inside, |out| out.shutdown(Shutdown::Write)));
}

/// Bytes from `from` to `to` until either ends, then `to` shut for
/// writing so the far side sees the end.
fn copied<R: Read, W: Write>(mut from: R, mut to: W, shut: impl FnOnce(&W) -> std::io::Result<()>) {
    if let Err(error) = std::io::copy(&mut from, &mut to) {
        eprintln!("api-relay-inner: a relayed connection ended with an error: {error}");
    }
    // the far side may have closed already; its end is what was wanted
    let _ = shut(&to);
}
