use eframe::egui;
use std::time::Instant;

use crate::{explorer::ExplorerView, piano_roll::PianoRollView, playlist::PlaylistView, theme};

const APP_TOOLBAR_HEIGHT: f32 = 46.0;
const EXPLORER_WIDTH: f32 = 272.0;
const DEFAULT_BPM: f32 = 120.0;
const DEFAULT_LOOP_BARS: f32 = 4.0;

pub struct DawUiApp {
    explorer: ExplorerView,
    piano_roll: PianoRollView,
    piano_roll_open: bool,
    playlist: PlaylistView,
    transport: UiTransport,
}

struct UiTransport {
    bpm: f32,
    loop_length_beats: f32,
    playhead_beats: f32,
    playing: bool,
    last_tick: Option<Instant>,
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
            transport: UiTransport::new(),
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
                if toolbar_button(ui, "Play", self.transport.playing).clicked() {
                    self.transport.toggle_playback();
                }
                if toolbar_button(ui, "Stop", false).clicked() {
                    self.transport.stop();
                }

                toolbar_chip(
                    ui,
                    &format!("Beat {:.2}", self.transport.playhead_beats + 1.0),
                );
                toolbar_chip(
                    ui,
                    &format!("Loop {:.0} Bars", self.transport.loop_length_beats / 4.0),
                );
                toolbar_chip(ui, &format!("{:.0} BPM", self.transport.bpm));
                toolbar_chip(ui, "Snap 1/4");
            });
        });
    }
}

impl eframe::App for DawUiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.transport.advance(ctx);

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

                let mut active_track = self.playlist.active_track_snapshot();
                let piano_roll_tracks = self.playlist.piano_roll_tracks();
                let piano_roll_output = self.piano_roll.show_window(
                    ctx,
                    &mut self.piano_roll_open,
                    &mut active_track,
                    &piano_roll_tracks,
                    self.transport.playhead_beats,
                );
                if let Some(seek_to_beat) = piano_roll_output.seek_to_beat {
                    self.transport.seek(seek_to_beat);
                }
                self.playlist.set_active_track_notes(active_track.notes);

                ui.scope_builder(egui::UiBuilder::new().max_rect(workspace_rect), |ui| {
                    if let Some(seek_to_beat) = self.playlist.show(
                        ui,
                        !piano_roll_output.blocks_pointer,
                        self.transport.playhead_beats,
                    ) {
                        self.transport.seek(seek_to_beat);
                    }
                });
            });
    }
}

impl UiTransport {
    fn new() -> Self {
        Self {
            bpm: DEFAULT_BPM,
            loop_length_beats: DEFAULT_LOOP_BARS * 4.0,
            playhead_beats: 0.0,
            playing: false,
            last_tick: None,
        }
    }

    fn toggle_playback(&mut self) {
        self.playing = !self.playing;
        self.last_tick = Some(Instant::now());
    }

    fn stop(&mut self) {
        self.playing = false;
        self.playhead_beats = 0.0;
        self.last_tick = None;
    }

    fn seek(&mut self, beats: f32) {
        let loop_end = self.loop_length_beats.max(1.0);
        self.playhead_beats = beats.clamp(0.0, loop_end);
        self.last_tick = Some(Instant::now());
    }

    fn advance(&mut self, ctx: &egui::Context) {
        if !self.playing {
            self.last_tick = None;
            return;
        }

        let now = Instant::now();
        let delta_seconds = self
            .last_tick
            .map(|last_tick| (now - last_tick).as_secs_f32())
            .unwrap_or_default();
        self.last_tick = Some(now);

        if delta_seconds > 0.0 {
            self.playhead_beats += delta_seconds * self.bpm / 60.0;
            let loop_end = self.loop_length_beats.max(1.0);
            while self.playhead_beats > loop_end {
                self.playhead_beats -= loop_end;
            }
        }

        ctx.request_repaint();
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

fn toolbar_chip(ui: &mut egui::Ui, label: &str) -> egui::Response {
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
