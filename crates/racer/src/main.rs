//! Native launch for the same racer that runs on the web.
fn main() {
    if let Err(error) = vapors_racer::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
