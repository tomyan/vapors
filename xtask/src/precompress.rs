//! The release page precompressed: beside each text or wasm file, a
//! brotli copy (`.br`) and a gzip one (`.gz`), which static-web-server
//! serves by the request's `Accept-Encoding` (`--compression-static`).
//! The phone's download was the page's slowest stage (Tom, 2026-10-26:
//! 1.2 MB of the 2.8 MB wasm in 60 s); compressed, it moves about a
//! third of that.

use std::io::Write as _;
use std::path::{Path, PathBuf};

/// The extensions compressed: the page's own files and its wasm.
/// Images are compressed already; a `.br` or `.gz` is never compressed
/// again.
const COMPRESSED: [&str; 7] = ["wasm", "js", "css", "html", "json", "webmanifest", "svg"];

/// brotli quality 9: the highest quality that leaves the `web` job well
/// inside its 3-minute limit on GitHub's runner, which compresses about
/// 4-5x slower than the dev box (lead, 2026-10-27). Quality 11 put the
/// job past the limit: the trunk step went from 32 s to 74-96 s, all of
/// it compression, 42-64 s on the runner for the 12 s measured here.
/// Quality 10 saves only about 28% of that; 9 is about 8x faster than
/// 11. Measured 2026-10-27 on the dev box (i5-10500T, niced, `brotli`
/// 9.1.0, window 22), the wasm of each page alone; the other files add
/// under 0.2 s at any quality:
///
/// | quality | vapors demo wasm, 7,261,627 B | touche page wasm, 2,875,921 B |
/// |---------|-------------------------------|-------------------------------|
/// | 9       |  1.55 s   3,405,449 B         | 0.36 s   859,189 B            |
/// | 10      |  8.68 s   3,174,260 B         | 3.05 s   784,325 B            |
/// | 11      | 12.04 s   3,062,894 B         | 4.97 s   757,768 B            |
/// | gzip 9  |  0.48 s   3,896,795 B         | 0.21 s 1,041,805 B            |
///
/// Qualities 5 to 8 took 0.15 to 1.2 s and were 1 to 2.5% larger than 9.
/// The page wasm at 9 is 839 KiB, within touche's 900 KiB budget
/// (`dist_transfer.rs`). Raising the quality is a change with a new
/// measurement against the `web` job's limit.
const BROTLI_QUALITY: u32 = 9;

/// A 4 MiB window (lgwin 22). The RFC's largest, 24, was within noise
/// on the same measurement: at quality 9, the same size and time on the
/// demo wasm; identical on the page wasm, which fits in 4 MiB.
const BROTLI_WINDOW: u32 = 22;

/// What one file became.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compressed {
    pub file: PathBuf,
    pub bytes: u64,
    pub brotli: u64,
    pub gzip: u64,
}

/// Write `.br` and `.gz` beside every file in `dir` (at any depth) whose
/// extension is in [`COMPRESSED`].
///
/// # Errors
///
/// A file unread or unwritten, naming it.
pub fn precompress(dir: &Path) -> Result<Vec<Compressed>, String> {
    files(dir)?
        .into_iter()
        .filter(|file| compressible(file))
        .map(|file| one(&file))
        .collect()
}

fn compressible(file: &Path) -> bool {
    file.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| COMPRESSED.iter().any(|c| c.eq_ignore_ascii_case(e)))
}

fn one(file: &Path) -> Result<Compressed, String> {
    let named = |e: std::io::Error| format!("{}: {e}", file.display());
    let bytes = std::fs::read(file).map_err(named)?;
    let brotli = brotli_of(&bytes).map_err(named)?;
    let gzip = gzip_of(&bytes).map_err(named)?;
    std::fs::write(beside(file, "br"), &brotli).map_err(named)?;
    std::fs::write(beside(file, "gz"), &gzip).map_err(named)?;
    Ok(Compressed {
        file: file.to_path_buf(),
        bytes: bytes.len() as u64,
        brotli: brotli.len() as u64,
        gzip: gzip.len() as u64,
    })
}

/// `file` with `.<extension>` added: `a.wasm` becomes `a.wasm.br`.
pub fn beside(file: &Path, extension: &str) -> PathBuf {
    let mut name = file.as_os_str().to_owned();
    name.push(".");
    name.push(extension);
    PathBuf::from(name)
}

fn brotli_of(bytes: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut out = Vec::new();
    {
        let mut writer =
            brotli::CompressorWriter::new(&mut out, 1 << 16, BROTLI_QUALITY, BROTLI_WINDOW);
        writer.write_all(bytes)?;
    }
    Ok(out)
}

fn gzip_of(bytes: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut writer = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    writer.write_all(bytes)?;
    writer.finish()
}

fn files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
        if path.is_dir() {
            found.extend(files(&path)?);
        } else {
            found.push(path);
        }
    }
    Ok(found)
}

#[cfg(test)]
#[path = "precompress_tests.rs"]
mod tests;
