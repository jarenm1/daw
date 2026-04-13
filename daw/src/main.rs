// Minimal app entry point while the UI is being rebuilt.
fn main() {
    if let Err(error) = daw_ui::run() {
        eprintln!("failed to launch DAW UI: {error}");
        std::process::exit(1);
    }
}
