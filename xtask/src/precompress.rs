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

/// brotli's best quality and a 4 MiB window (its largest without the
/// large-window extension browsers do not read).
const BROTLI_QUALITY: u32 = 11;
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
