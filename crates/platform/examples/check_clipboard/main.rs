//! The X11 clipboard's image through [`SystemClipboard`], set by `xclip`
//! as another program would: `xvfb-run -a cargo run -p vapors-platform
//! --example check_clipboard` (the CI window job runs it). A PNG on the
//! clipboard reads back as its pixels, and prints how long the read
//! took; text only reads as no image (pinning the X11 wrong-type case
//! clipboard.rs matches); bytes that are not a PNG, a selection owner
//! that stops answering, and a display that cannot be reached read as
//! unreadable. Exit 1 names what differed; with no display or no xclip
//! it fails naming the fix. An image over the size limit is a unit test
//! (`clipboard_tests.rs`): arboard decodes the whole image before it can
//! be refused.

use std::path::Path;
use std::process::{Command, ExitCode};
use std::time::{Duration, Instant};

mod xclip;

use vapors_platform::{CLIPBOARD_DEADLINE, Clipboard, ClipboardImage, SystemClipboard};
use xclip::{newest_xclip, signal, xclip};

const REPRODUCE: &str = "xvfb-run -a cargo run -p vapors-platform --example check_clipboard";
const FIXTURE: &str = "examples/fixtures/gradient.png";
/// The flag the unreachable-display case runs this program again with.
const UNREACHABLE: &str = "--read-on-this-display";

fn main() -> ExitCode {
    if std::env::args().any(|a| a == UNREACHABLE) {
        println!("{}", short(&SystemClipboard.image()));
        return ExitCode::SUCCESS;
    }
    match run() {
        Ok(lines) => {
            for line in &lines {
                println!("ok   {line}");
            }
            ExitCode::SUCCESS
        }
        Err(problem) => {
            eprintln!("check_clipboard: {problem}\nreproduce: {REPRODUCE}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<Vec<String>, String> {
    if std::env::var("DISPLAY").unwrap_or_default().is_empty() {
        return Err("DISPLAY is unset; fix: run under `xvfb-run -a`".to_owned());
    }
    Ok(vec![
        png_reads_back()?,
        text_is_no_image()?,
        corrupt_is_unreadable()?,
        stopped_owner()?,
        unreachable_display()?,
    ])
}

fn png_reads_back() -> Result<String, String> {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    xclip(&["-t", "image/png", "-i"], Some(&fixture), None)?;
    let (width, height, rgba) = decode(&fixture)?;
    let started = Instant::now();
    let image = SystemClipboard.image();
    let took = started.elapsed().as_millis();
    match image {
        ClipboardImage::Image {
            width: w,
            height: h,
            rgba: read,
        } if (w, h) == (width, height) && read == rgba => Ok(format!(
            "a {width}x{height} PNG set by xclip reads back as its pixels ({took} ms)"
        )),
        other => Err(format!(
            "the PNG xclip set read as {:?}, not its {width}x{height} pixels",
            short(&other)
        )),
    }
}

fn text_is_no_image() -> Result<String, String> {
    xclip(&["-t", "UTF8_STRING"], None, Some(b"warmer"))?;
    match SystemClipboard.image() {
        ClipboardImage::NoImage => Ok("text alone on the clipboard reads as no image".to_owned()),
        other => Err(format!(
            "text alone read as {:?}, not NoImage",
            short(&other)
        )),
    }
}

fn corrupt_is_unreadable() -> Result<String, String> {
    xclip(&["-t", "image/png"], None, Some(b"\x89PNG not a png"))?;
    match SystemClipboard.image() {
        ClipboardImage::Unreadable(_) => {
            Ok("bytes offered as a PNG that are not one read as unreadable".to_owned())
        }
        other => Err(format!(
            "a corrupt PNG read as {:?}, not Unreadable",
            short(&other)
        )),
    }
}

/// xclip's serving child stopped (SIGSTOP): the owner exists and never
/// answers. Refused within the deadline, then the child is let go.
fn stopped_owner() -> Result<String, String> {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    xclip(&["-t", "image/png", "-i"], Some(&fixture), None)?;
    let owner = newest_xclip()?;
    signal(&owner, "STOP")?;
    let started = Instant::now();
    let read = SystemClipboard.image();
    let took = started.elapsed();
    signal(&owner, "KILL")?;
    match read {
        ClipboardImage::Unreadable(_)
            if took <= CLIPBOARD_DEADLINE + Duration::from_millis(500) =>
        {
            Ok(format!(
                "an owner that stops answering reads as unreadable, after {} ms",
                took.as_millis()
            ))
        }
        other => Err(format!(
            "a stopped owner read as {:?} after {} ms",
            short(&other),
            took.as_millis()
        )),
    }
}

/// This program again, on a display nobody serves.
fn unreachable_display() -> Result<String, String> {
    let me = std::env::current_exe().map_err(|e| e.to_string())?;
    let output = Command::new(me)
        .arg(UNREACHABLE)
        .env("DISPLAY", ":9999")
        .output()
        .map_err(|e| format!("running the unreachable-display case: {e}"))?;
    let said = String::from_utf8_lossy(&output.stdout);
    if said.starts_with("Unreadable") {
        Ok("a display that cannot be reached reads as unreadable, not as no image".to_owned())
    } else {
        Err(format!(
            "on an unreachable display the clipboard read as {said}"
        ))
    }
}

fn decode(path: &Path) -> Result<(u32, u32, Vec<u8>), String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .map_err(|e| e.to_string())?;
    let mut rgba = vec![0; reader.output_buffer_size().ok_or("no buffer size")?];
    let info = reader.next_frame(&mut rgba).map_err(|e| e.to_string())?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return Err(format!("{FIXTURE} is not 8-bit RGBA"));
    }
    rgba.truncate(info.buffer_size());
    Ok((info.width, info.height, rgba))
}

/// The answer without its pixels, for a message.
fn short(read: &ClipboardImage) -> String {
    match read {
        ClipboardImage::Image { width, height, .. } => format!("Image {width}x{height}"),
        other => format!("{other:?}"),
    }
}
