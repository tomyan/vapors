//! The X11 clipboard's image through [`SystemClipboard`], set by `xclip`
//! as another program would: `xvfb-run -a cargo run -p vapors-platform
//! --example check_clipboard` (the CI window job runs it). A PNG on the
//! clipboard reads back as its pixels; text only reads as no image; a
//! display that cannot be reached reads as unreadable. Exit 1 names
//! what differed; with no display or no xclip it fails naming the fix.

use std::path::Path;
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

use vapors_platform::{Clipboard, ClipboardImage, SystemClipboard};

const REPRODUCE: &str = "xvfb-run -a cargo run -p vapors-platform --example check_clipboard";
const FIXTURE: &str = "examples/fixtures/gradient.png";
/// The flag the unreachable-display case runs this program again with.
const UNREACHABLE: &str = "--read-on-this-display";
/// xclip forks and serves the selection; its parent exits at once
/// (under 10 ms here). 5 s is the deadline for it to say it has.
const XCLIP_DEADLINE: Duration = Duration::from_secs(5);

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
        unreachable_display()?,
    ])
}

fn png_reads_back() -> Result<String, String> {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    xclip(&["-t", "image/png", "-i"], Some(&fixture), None)?;
    let (width, height, rgba) = decode(&fixture)?;
    match SystemClipboard.image() {
        ClipboardImage::Image {
            width: w,
            height: h,
            rgba: read,
        } if (w, h) == (width, height) && read == rgba => Ok(format!(
            "a {width}x{height} PNG set by xclip reads back as its pixels"
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

/// `xclip -selection clipboard <args>`, given a file or bytes on stdin;
/// refused naming the fix when xclip is missing or does not finish.
fn xclip(args: &[&str], file: Option<&Path>, stdin: Option<&[u8]>) -> Result<(), String> {
    let mut command = Command::new("xclip");
    command.args(["-selection", "clipboard"]).args(args);
    if let Some(file) = file {
        command.arg(file);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .map_err(|e| format!("xclip: {e}; fix: install xclip (apt install xclip)"))?;
    if let (Some(bytes), Some(mut pipe)) = (stdin, child.stdin.take()) {
        std::io::Write::write_all(&mut pipe, bytes).map_err(|e| format!("xclip's stdin: {e}"))?;
    }
    drop(child.stdin.take());
    wait(&mut child)
}

fn wait(child: &mut std::process::Child) -> Result<(), String> {
    let started = Instant::now();
    while started.elapsed() < XCLIP_DEADLINE {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return if status.success() {
                Ok(())
            } else {
                Err(format!("xclip exited {status}"))
            };
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let _ = child.kill();
    Err(format!("xclip did not finish within {XCLIP_DEADLINE:?}"))
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
