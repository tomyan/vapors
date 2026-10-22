//! Around the Chromium process: the deadline, its output, the files it
//! reads and writes, and where its binary is.

use std::path::{Path, PathBuf};
use std::process::{Child, ExitStatus};
use std::time::{Duration, Instant};

use super::chromium::ChromiumError;
use super::measure::Rgba;

/// Chromium here starts and screenshots a page in about 1 s; a cold CI
/// runner was measured at under 10 s. 60 s is the ceiling before the run
/// is declared hung.
const DEADLINE: Duration = Duration::from_secs(60);

/// Delete an earlier screenshot so a stale image can never be read back.
pub(crate) fn remove_stale(png: &Path) -> Result<(), ChromiumError> {
    match std::fs::remove_file(png) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(ChromiumError::Failed(format!(
            "cannot remove the previous {}: {e}",
            png.display()
        ))),
    }
}

pub(crate) fn check_size(image: &Rgba, png: &Path, wanted: [u32; 2]) -> Result<(), ChromiumError> {
    if [image.width, image.height] == wanted {
        return Ok(());
    }
    Err(ChromiumError::Failed(format!(
        "{} is {}x{}, wanted {}x{}. Fix: the window size or scale was not honoured; \
         check --force-device-scale-factor and that no display scaling applies",
        png.display(),
        image.width,
        image.height,
        wanted[0],
        wanted[1]
    )))
}

pub(crate) fn wait_with_deadline(child: &mut Child) -> Result<ExitStatus, ChromiumError> {
    let started = Instant::now();
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|e| ChromiumError::Failed(format!("waiting: {e}")))?
        {
            return Ok(status);
        }
        if started.elapsed() > DEADLINE {
            let _ = child.kill();
            return Err(ChromiumError::Failed(format!(
                "did not finish within {}s; killed",
                DEADLINE.as_secs()
            )));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

pub(crate) fn read_all(mut stream: impl std::io::Read) -> String {
    let mut text = String::new();
    let _ = stream.read_to_string(&mut text);
    text
}

pub(crate) fn on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}
