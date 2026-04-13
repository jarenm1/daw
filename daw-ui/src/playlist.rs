use eframe::egui;

use crate::theme;

const TOOLBAR_HEIGHT: f32 = 38.0;
const TIMELINE_HEIGHT: f32 = 34.0;
const TRACK_HEADER_WIDTH: f32 = 164.0;
const TRACK_HEIGHT: f32 = 72.0;
const TRACK_GAP: f32 = 6.0;
const BAR_WIDTH: f32 = 120.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrackKind {
    PianoRoll,
    Audio,
}

#[derive(Clone, Debug)]
pub struct PianoRollNoteData {
    pub lane: usize,
    pub start_beat: f32,
    pub length_beats: f32,
    pub velocity: f32,
}

#[derive(Clone, Debug)]
pub struct PianoRollTrackSnapshot {
    pub track_index: usize,
    pub name: String,
    pub lane_label: String,
    pub kind: TrackKind,
    pub notes: Vec<PianoRollNoteData>,
}

pub struct PlaylistView {
    active_tool: PlaylistTool,
    selected_track: usize,
    tracks: Vec<PlaylistTrack>,
}

impl PlaylistView {
    pub fn new() -> Self {
        Self {
            active_tool: PlaylistTool::Select,
            selected_track: 0,
            tracks: sample_tracks(),
        }
    }

    pub fn active_track_name(&self) -> &str {
        self.tracks
            .get(self.selected_track)
            .map(|track| track.name)
            .unwrap_or("Playlist Track")
    }

    pub fn active_track_snapshot(&self) -> PianoRollTrackSnapshot {
        self.tracks
            .get(self.selected_track)
            .map(|track| track.snapshot(self.selected_track))
            .unwrap_or_else(|| PianoRollTrackSnapshot {
                track_index: 0,
                name: "Playlist Track".to_owned(),
                lane_label: "Piano roll lane".to_owned(),
                kind: TrackKind::PianoRoll,
                notes: Vec::new(),
            })
    }

    pub fn piano_roll_tracks(&self) -> Vec<PianoRollTrackSnapshot> {
        self.tracks
            .iter()
            .enumerate()
            .filter(|(_, track)| track.kind == TrackKind::PianoRoll)
            .map(|(index, track)| track.snapshot(index))
            .collect()
    }

    pub fn set_active_track_notes(&mut self, notes: Vec<PianoRollNoteData>) {
        if let Some(track) = self.tracks.get_mut(self.selected_track) {
            if track.kind == TrackKind::PianoRoll {
                track.notes = notes;
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        let frame = egui::Frame::default()
            .fill(theme::SURFACE_0)
            .corner_radius(egui::CornerRadius::ZERO)
            .inner_margin(egui::Margin::ZERO);

        frame.show(ui, |ui| {
            self.show_toolbar(ui);
            self.show_body(ui);
        });

        let stroke = egui::Stroke::new(1.0, theme::BORDER);
        let painter = ui.painter();
        painter.line_segment([rect.left_top(), rect.left_bottom()], stroke);
        painter.line_segment([rect.right_top(), rect.right_bottom()], stroke);
        painter.line_segment([rect.left_bottom(), rect.right_bottom()], stroke);
    }

    fn show_toolbar(&mut self, ui: &mut egui::Ui) {
        let width = ui.available_width();
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(width, TOOLBAR_HEIGHT), egui::Sense::hover());
        let painter = ui.painter();

        painter.rect_filled(rect, egui::CornerRadius::ZERO, theme::SURFACE_1);
        painter.line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            egui::Stroke::new(1.0, theme::BORDER),
        );
        painter.line_segment(
            [
                egui::pos2(rect.left(), rect.bottom() - 1.0),
                egui::pos2(rect.right(), rect.bottom() - 1.0),
            ],
            egui::Stroke::new(1.0, theme::GRID_ROW),
        );
        painter.line_segment(
            [
                egui::pos2(rect.left() + TRACK_HEADER_WIDTH, rect.top()),
                egui::pos2(rect.left() + TRACK_HEADER_WIDTH, rect.bottom()),
            ],
            egui::Stroke::new(1.0, theme::BORDER),
        );

        let toolbar_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left() + TRACK_HEADER_WIDTH + 8.0, rect.top() + 6.0),
            egui::pos2(rect.right() - 8.0, rect.bottom() - 6.0),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(toolbar_rect), |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;

                for tool in PlaylistTool::ALL {
                    let response = tool_button(ui, tool, self.active_tool == tool);
                    if response.clicked() {
                        self.active_tool = tool;
                    }
                }
            });
        });
    }

    fn show_body(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size_before_wrap();
        let (rect, _) = ui.allocate_exact_size(available, egui::Sense::hover());
        let painter = ui.painter().clone();

        painter.rect_filled(rect, egui::CornerRadius::ZERO, theme::SURFACE_0);

        let timeline_rect =
            egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), TIMELINE_HEIGHT));
        let track_area_rect =
            egui::Rect::from_min_max(egui::pos2(rect.left(), timeline_rect.bottom()), rect.max);
        let sidebar_rect = egui::Rect::from_min_max(
            egui::pos2(track_area_rect.left(), track_area_rect.top()),
            egui::pos2(
                track_area_rect.left() + TRACK_HEADER_WIDTH,
                track_area_rect.bottom(),
            ),
        );
        let grid_rect = egui::Rect::from_min_max(
            egui::pos2(sidebar_rect.right(), track_area_rect.top()),
            track_area_rect.max,
        );

        painter.rect_filled(timeline_rect, egui::CornerRadius::ZERO, theme::SURFACE_1);
        painter.rect_filled(sidebar_rect, egui::CornerRadius::ZERO, theme::SURFACE_1);
        painter.line_segment(
            [
                egui::pos2(sidebar_rect.right(), rect.top()),
                egui::pos2(sidebar_rect.right(), rect.bottom()),
            ],
            egui::Stroke::new(1.0, theme::BORDER),
        );
        painter.line_segment(
            [
                egui::pos2(rect.left(), timeline_rect.bottom()),
                egui::pos2(rect.right(), timeline_rect.bottom()),
            ],
            egui::Stroke::new(1.0, theme::BORDER),
        );

        draw_timeline(&painter, timeline_rect, TRACK_HEADER_WIDTH);
        draw_grid(&painter, grid_rect, self.tracks.len());
        draw_tracks(
            ui,
            &painter,
            sidebar_rect,
            grid_rect,
            &self.tracks,
            &mut self.selected_track,
        );
    }
}

impl Default for PlaylistView {
    fn default() -> Self {
        Self::new()
    }
}

struct PlaylistTrack {
    name: &'static str,
    lane_label: &'static str,
    kind: TrackKind,
    notes: Vec<PianoRollNoteData>,
}

impl PlaylistTrack {
    fn piano_roll(
        name: &'static str,
        lane_label: &'static str,
        notes: Vec<PianoRollNoteData>,
    ) -> Self {
        Self {
            name,
            lane_label,
            kind: TrackKind::PianoRoll,
            notes,
        }
    }

    fn audio(name: &'static str, lane_label: &'static str) -> Self {
        Self {
            name,
            lane_label,
            kind: TrackKind::Audio,
            notes: Vec::new(),
        }
    }

    fn snapshot(&self, track_index: usize) -> PianoRollTrackSnapshot {
        PianoRollTrackSnapshot {
            track_index,
            name: self.name.to_owned(),
            lane_label: self.lane_label.to_owned(),
            kind: self.kind,
            notes: self.notes.clone(),
        }
    }
}

fn sample_tracks() -> Vec<PlaylistTrack> {
    vec![
        PlaylistTrack::piano_roll(
            "Drums",
            "Drum pattern lane",
            vec![
                note(15, 0.0, 0.5, 0.92),
                note(15, 1.0, 0.5, 0.88),
                note(12, 2.0, 0.5, 0.80),
                note(15, 3.0, 0.5, 0.90),
                note(10, 4.5, 0.5, 0.74),
                note(12, 6.0, 0.5, 0.82),
            ],
        ),
        PlaylistTrack::piano_roll(
            "Bass",
            "Piano roll lane",
            vec![
                note(13, 0.0, 1.0, 0.84),
                note(11, 1.5, 0.75, 0.79),
                note(8, 3.0, 1.25, 0.88),
                note(10, 5.0, 1.0, 0.81),
                note(6, 6.5, 1.0, 0.76),
            ],
        ),
        PlaylistTrack::piano_roll(
            "Keys",
            "Chord lane",
            vec![
                note(12, 0.0, 1.5, 0.74),
                note(10, 0.0, 1.5, 0.71),
                note(8, 0.0, 1.5, 0.68),
                note(11, 2.0, 1.5, 0.75),
                note(9, 2.0, 1.5, 0.70),
                note(7, 2.0, 1.5, 0.67),
                note(10, 4.0, 2.0, 0.78),
                note(7, 4.0, 2.0, 0.69),
                note(5, 4.0, 2.0, 0.65),
            ],
        ),
        PlaylistTrack::piano_roll(
            "Lead",
            "Melody lane",
            vec![
                note(5, 0.5, 0.75, 0.86),
                note(6, 1.5, 0.5, 0.81),
                note(8, 2.25, 1.0, 0.91),
                note(7, 4.0, 0.5, 0.80),
                note(9, 5.0, 0.75, 0.89),
                note(11, 6.0, 1.25, 0.94),
            ],
        ),
        PlaylistTrack::audio("Vox", "Audio lane"),
    ]
}

fn note(lane: usize, start_beat: f32, length_beats: f32, velocity: f32) -> PianoRollNoteData {
    PianoRollNoteData {
        lane,
        start_beat,
        length_beats,
        velocity,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlaylistTool {
    Select,
    Draw,
    Split,
    Duplicate,
    Mute,
}

impl PlaylistTool {
    pub const ALL: [Self; 5] = [
        Self::Select,
        Self::Draw,
        Self::Split,
        Self::Duplicate,
        Self::Mute,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::Draw => "Draw",
            Self::Split => "Split",
            Self::Duplicate => "Duplicate",
            Self::Mute => "Mute",
        }
    }
}

fn tool_button(ui: &mut egui::Ui, tool: PlaylistTool, active: bool) -> egui::Response {
    let size = egui::vec2(28.0, 24.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let painter = ui.painter();

    let fill = if active {
        theme::TOOL_ACTIVE_WASH
    } else {
        egui::Color32::TRANSPARENT
    };

    let stroke_color = if active {
        theme::ACCENT
    } else if response.hovered() {
        theme::ACCENT_HOVER
    } else {
        theme::TEXT_MUTED
    };

    if fill != egui::Color32::TRANSPARENT {
        painter.rect_filled(rect, egui::CornerRadius::ZERO, fill);
    }

    let icon_rect = rect.shrink2(egui::vec2(6.0, 5.0));
    let icon_stroke = egui::Stroke::new(
        1.6,
        if active {
            theme::ACCENT_HOVER
        } else if response.hovered() {
            theme::TEXT
        } else {
            theme::TEXT_MUTED
        },
    );
    paint_tool_icon(painter, icon_rect, tool, icon_stroke);

    if active {
        painter.line_segment(
            [
                egui::pos2(rect.left(), rect.bottom()),
                egui::pos2(rect.right(), rect.bottom()),
            ],
            egui::Stroke::new(2.0, stroke_color),
        );
    }

    response.on_hover_text(tool.label())
}

fn paint_tool_icon(
    painter: &egui::Painter,
    rect: egui::Rect,
    tool: PlaylistTool,
    stroke: egui::Stroke,
) {
    match tool {
        PlaylistTool::Select => {
            let points = vec![
                egui::pos2(rect.left(), rect.top()),
                egui::pos2(rect.left() + rect.width() * 0.68, rect.center().y),
                egui::pos2(rect.center().x, rect.center().y),
                egui::pos2(rect.right(), rect.bottom()),
                egui::pos2(rect.center().x + 1.0, rect.bottom()),
                egui::pos2(rect.left(), rect.top()),
            ];
            painter.add(egui::Shape::line(points, stroke));
        }
        PlaylistTool::Draw => {
            painter.line_segment(
                [
                    egui::pos2(rect.left(), rect.bottom()),
                    egui::pos2(rect.right(), rect.top()),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(rect.right() - 3.0, rect.top() + 1.0),
                    egui::pos2(rect.right(), rect.top() + 4.0),
                ],
                stroke,
            );
        }
        PlaylistTool::Split => {
            let x = rect.center().x;
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(rect.left(), rect.top() + 3.0),
                    egui::pos2(x - 2.0, rect.center().y),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(rect.left(), rect.bottom() - 3.0),
                    egui::pos2(x - 2.0, rect.center().y),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(rect.right(), rect.top() + 3.0),
                    egui::pos2(x + 2.0, rect.center().y),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(rect.right(), rect.bottom() - 3.0),
                    egui::pos2(x + 2.0, rect.center().y),
                ],
                stroke,
            );
        }
        PlaylistTool::Duplicate => {
            let back = egui::Rect::from_min_max(
                egui::pos2(rect.left() + 1.0, rect.top() + 3.0),
                egui::pos2(rect.right() - 4.0, rect.bottom() - 1.0),
            );
            let front = back.translate(egui::vec2(4.0, -3.0));
            painter.rect_stroke(
                back,
                egui::CornerRadius::ZERO,
                stroke,
                egui::StrokeKind::Outside,
            );
            painter.rect_stroke(
                front,
                egui::CornerRadius::ZERO,
                stroke,
                egui::StrokeKind::Outside,
            );
        }
        PlaylistTool::Mute => {
            let speaker = vec![
                egui::pos2(rect.left(), rect.center().y - 2.0),
                egui::pos2(rect.left() + 3.0, rect.center().y - 2.0),
                egui::pos2(rect.left() + 6.0, rect.top() + 1.0),
                egui::pos2(rect.left() + 6.0, rect.bottom() - 1.0),
                egui::pos2(rect.left() + 3.0, rect.center().y + 2.0),
                egui::pos2(rect.left(), rect.center().y + 2.0),
            ];
            painter.add(egui::Shape::closed_line(speaker, stroke));
            painter.line_segment(
                [
                    egui::pos2(rect.center().x + 1.0, rect.top() + 2.0),
                    egui::pos2(rect.right(), rect.bottom() - 2.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(rect.right(), rect.top() + 2.0),
                    egui::pos2(rect.center().x + 1.0, rect.bottom() - 2.0),
                ],
                stroke,
            );
        }
    }
}

fn draw_timeline(painter: &egui::Painter, rect: egui::Rect, offset_x: f32) {
    let bars = ((rect.width() - offset_x) / BAR_WIDTH).ceil().max(1.0) as usize;
    for index in 0..bars {
        let x = rect.left() + offset_x + index as f32 * BAR_WIDTH;
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(1.0, theme::GRID_MAJOR),
        );
        painter.text(
            egui::pos2(x + 10.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            format!("{:02}", index + 1),
            egui::FontId::proportional(12.0),
            theme::TEXT_MUTED,
        );
    }
}

fn draw_grid(painter: &egui::Painter, rect: egui::Rect, track_count: usize) {
    let bars = (rect.width() / BAR_WIDTH).ceil().max(1.0) as usize;
    for index in 0..=bars * 4 {
        let x = rect.left() + index as f32 * (BAR_WIDTH / 4.0);
        let stroke = if index % 4 == 0 {
            egui::Stroke::new(1.0, theme::GRID_MAJOR)
        } else {
            egui::Stroke::new(1.0, theme::GRID_MINOR)
        };
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            stroke,
        );
    }

    for index in 0..=track_count {
        let y = rect.top() + index as f32 * (TRACK_HEIGHT + TRACK_GAP);
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(1.0, theme::GRID_ROW),
        );
    }
}

fn draw_tracks(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    sidebar_rect: egui::Rect,
    grid_rect: egui::Rect,
    tracks: &[PlaylistTrack],
    selected_track: &mut usize,
) {
    for (index, track) in tracks.iter().enumerate() {
        let top = sidebar_rect.top() + index as f32 * (TRACK_HEIGHT + TRACK_GAP);
        let track_rect = egui::Rect::from_min_max(
            egui::pos2(sidebar_rect.left(), top),
            egui::pos2(sidebar_rect.right(), top + TRACK_HEIGHT),
        );
        let clip_rect = egui::Rect::from_min_max(
            egui::pos2(grid_rect.left() + 20.0 + index as f32 * 38.0, top + 14.0),
            egui::pos2(grid_rect.left() + 216.0 + index as f32 * 44.0, top + 56.0),
        );
        let clip_rect = egui::Rect::from_min_max(
            clip_rect.min,
            egui::pos2(
                clip_rect.max.x.min(grid_rect.right() - 12.0),
                clip_rect.max.y,
            ),
        );

        let row_response = ui.interact(
            track_rect.union(clip_rect),
            ui.make_persistent_id(("playlist_track", index)),
            egui::Sense::click(),
        );
        if row_response.clicked() {
            *selected_track = index;
        }

        let is_selected = *selected_track == index;
        let track_fill = if is_selected {
            theme::TOOL_ACTIVE_WASH
        } else if index % 2 == 0 {
            theme::SURFACE_1
        } else {
            theme::SURFACE_0
        };

        painter.rect_filled(track_rect, egui::CornerRadius::ZERO, track_fill);
        if is_selected {
            painter.line_segment(
                [
                    egui::pos2(track_rect.left(), track_rect.top()),
                    egui::pos2(track_rect.left(), track_rect.bottom()),
                ],
                egui::Stroke::new(2.0, theme::ACCENT),
            );
        }
        painter.text(
            egui::pos2(track_rect.left() + 14.0, track_rect.top() + 18.0),
            egui::Align2::LEFT_CENTER,
            track.name,
            egui::FontId::proportional(13.0),
            theme::TEXT,
        );
        painter.text(
            egui::pos2(track_rect.left() + 14.0, track_rect.top() + 40.0),
            egui::Align2::LEFT_CENTER,
            track.lane_label,
            egui::FontId::proportional(11.0),
            theme::TEXT_MUTED,
        );

        if clip_rect.width() > 6.0 {
            let clip_fill = if is_selected {
                egui::Color32::from_rgba_unmultiplied(111, 184, 255, 56)
            } else if track.kind == TrackKind::Audio {
                egui::Color32::from_rgba_unmultiplied(196, 202, 214, 32)
            } else {
                theme::accent_soft()
            };
            painter.rect_filled(clip_rect, egui::CornerRadius::ZERO, clip_fill);
            painter.rect_stroke(
                clip_rect,
                egui::CornerRadius::ZERO,
                egui::Stroke::new(
                    if is_selected { 1.5 } else { 1.0 },
                    if is_selected {
                        theme::ACCENT_HOVER
                    } else if track.kind == TrackKind::Audio {
                        theme::TEXT_MUTED
                    } else {
                        theme::ACCENT
                    },
                ),
                egui::StrokeKind::Outside,
            );
        }
    }
}
