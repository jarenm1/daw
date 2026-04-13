use eframe::egui;

use crate::{explorer::ExplorerView, piano_roll::PianoRollView, playlist::PlaylistView, theme};

const APP_TOOLBAR_HEIGHT: f32 = 46.0;
const EXPLORER_WIDTH: f32 = 272.0;

pub struct DawUiApp {
    explorer: ExplorerView,
    piano_roll: PianoRollView,
    piano_roll_open: bool,
    playlist: PlaylistView,
}

impl DawUiApp {
    pub fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&creation_context.egui_ctx);
        egui_extras::install_image_loaders(&creation_context.egui_ctx);
        Self {
            explorer: ExplorerView::new(),
            piano_roll: PianoRollView::new(),
            piano_roll_open: false,
            playlist: PlaylistView::new(),
        }
    }

    fn show_toolbar(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, egui::CornerRadius::ZERO, theme::SURFACE_1);
        painter.line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            egui::Stroke::new(1.0, theme::BORDER),
        );

        let inner = rect.shrink2(egui::vec2(12.0, 6.0));
        ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;

                ui.label(
                    egui::RichText::new("DAW")
                        .size(14.0)
                        .strong()
                        .color(theme::TEXT),
                );
                ui.label(
                    egui::RichText::new("Arrange")
                        .size(11.0)
                        .color(theme::TEXT_MUTED),
                );
                ui.add_space(12.0);

                if toolbar_button(ui, "Piano Roll", self.piano_roll_open).clicked() {
                    self.piano_roll_open = !self.piano_roll_open;
                }

                ui.add_space(14.0);
                toolbar_chip(ui, "Pattern 1");
                toolbar_chip(ui, "120 BPM");
                toolbar_chip(ui, "Snap 1/4");
            });
        });
    }
}

impl eframe::App for DawUiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(theme::APP_BACKGROUND)
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let toolbar_rect = egui::Rect::from_min_max(
                    rect.min,
                    egui::pos2(rect.right(), rect.top() + APP_TOOLBAR_HEIGHT),
                );
                let content_rect = egui::Rect::from_min_max(
                    egui::pos2(rect.left(), toolbar_rect.bottom()),
                    rect.max,
                );
                let explorer_width = EXPLORER_WIDTH.min((content_rect.width() * 0.4).max(220.0));
                let divider_x = content_rect.left() + explorer_width;
                let explorer_rect = egui::Rect::from_min_max(
                    content_rect.min,
                    egui::pos2(divider_x, content_rect.bottom()),
                );
                let workspace_rect = egui::Rect::from_min_max(
                    egui::pos2(divider_x, content_rect.top()),
                    content_rect.max,
                );

                self.show_toolbar(ui, toolbar_rect);

                ui.scope_builder(egui::UiBuilder::new().max_rect(explorer_rect), |ui| {
                    self.explorer.show(ui);
                });

                ui.scope_builder(egui::UiBuilder::new().max_rect(workspace_rect), |ui| {
                    self.playlist.show(ui);
                });
            });

        let mut active_track = self.playlist.active_track_snapshot();
        let piano_roll_tracks = self.playlist.piano_roll_tracks();
        self.piano_roll.show_window(
            ctx,
            &mut self.piano_roll_open,
            &mut active_track,
            &piano_roll_tracks,
        );
        self.playlist.set_active_track_notes(active_track.notes);
    }
}

fn toolbar_button(ui: &mut egui::Ui, label: &'static str, active: bool) -> egui::Response {
    let text = egui::RichText::new(label).size(11.5).color(if active {
        theme::TEXT
    } else {
        theme::TEXT_MUTED
    });
    let button = egui::Button::new(text)
        .min_size(egui::vec2(0.0, 28.0))
        .fill(if active {
            theme::TOOL_ACTIVE_WASH
        } else {
            theme::SURFACE_0
        })
        .stroke(egui::Stroke::new(
            1.0,
            if active { theme::ACCENT } else { theme::BORDER },
        ))
        .corner_radius(4.0);
    ui.add(button)
}

fn toolbar_chip(ui: &mut egui::Ui, label: &'static str) -> egui::Response {
    let text = egui::RichText::new(label)
        .size(10.5)
        .color(theme::TEXT_MUTED);
    let button = egui::Button::new(text)
        .min_size(egui::vec2(0.0, 24.0))
        .fill(theme::SURFACE_0)
        .stroke(egui::Stroke::new(1.0, theme::BORDER))
        .corner_radius(3.0);
    ui.add(button)
}
