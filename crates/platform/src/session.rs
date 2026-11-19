//! Which windowing system a window opens on, and which window options it
//! can honour. An option it cannot honour is refused, never ignored.

use crate::PlatformError;
use crate::window::WindowOptions;

/// The windowing system behind the event loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // each target builds only its own sessions; tests build them all
pub enum Session {
    Wayland,
    X11,
    /// macOS or Windows.
    Desktop,
    Web,
}

impl Session {
    fn name(self) -> &'static str {
        match self {
            Self::Wayland => "Wayland",
            Self::X11 => "X11",
            Self::Desktop => "this desktop platform (macOS or Windows)",
            Self::Web => "the web",
        }
    }

    fn honours_app_id(self) -> bool {
        self == Self::Wayland
    }

    fn honours_decorations(self) -> bool {
        matches!(self, Self::Wayland | Self::Desktop)
    }

    /// The session of a running event loop.
    #[cfg(not(target_family = "wasm"))]
    pub fn of(event_loop: &winit::event_loop::ActiveEventLoop) -> Self {
        #[cfg(any(
            target_os = "linux",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        {
            use winit::platform::wayland::ActiveEventLoopExtWayland;
            if event_loop.is_wayland() {
                return Self::Wayland;
            }
            return Self::X11;
        }
        #[allow(unreachable_code)]
        {
            let _ = event_loop;
            Self::Desktop
        }
    }
}

/// Refuses an option the session cannot honour, naming the session, the
/// option and the fix.
pub fn refuse_unhonoured(options: &WindowOptions, session: Session) -> Result<(), PlatformError> {
    if options.app_id.is_some() && !session.honours_app_id() {
        return Err(unhonoured(session, "app_id", "set `app_id: None`"));
    }
    if !options.decorations && !session.honours_decorations() {
        return Err(unhonoured(
            session,
            "decorations: false",
            "set `decorations: true`",
        ));
    }
    Ok(())
}

fn unhonoured(session: Session, option: &str, leave: &str) -> PlatformError {
    PlatformError::Windowing(format!(
        "the window option `{option}` is not honoured on {}. Fix: run under Wayland, or {leave}. \
         Reproduce: `cargo run -p vapors-gui-bench-app -- --scenario app` on this session.",
        session.name()
    ))
}
