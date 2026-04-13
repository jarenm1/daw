mod app;
pub mod explorer;
pub mod piano_roll;
pub mod playlist;
mod theme;

pub use app::DawUiApp;

pub fn run() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("DAW")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([960.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "DAW",
        options,
        Box::new(|creation_context| Ok(Box::new(DawUiApp::new(creation_context)))),
    )
}
