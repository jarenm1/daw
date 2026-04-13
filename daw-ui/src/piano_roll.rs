use std::collections::BTreeSet;

use eframe::egui;

use crate::{
    playlist::{PianoRollNoteData, PianoRollTrackSnapshot, TrackKind},
    theme,
};

const HEADER_HEIGHT: f32 = 38.0;
const TOOLBAR_HEIGHT: f32 = 38.0;
const KEYBOARD_WIDTH: f32 = 88.0;
const ROW_HEIGHT: f32 = 22.0;
const BEAT_WIDTH: f32 = 56.0;
const SNAP_BEATS: f32 = 0.25;
const MIN_NOTE_LENGTH: f32 = SNAP_BEATS;
const RESIZE_HANDLE_WIDTH: f32 = 8.0;
const NOTE_ROWS: usize = 18;
const TOP_MIDI_NOTE: i32 = 79;

pub struct PianoRollView {
    active_tool: PianoRollTool,
    ghost_notes_enabled: bool,
    ghost_track_indices: BTreeSet<usize>,
    note_drag: Option<NoteDrag>,
}

#[derive(Clone, Copy, Debug)]
struct NoteDrag {
    track_index: usize,
    note_index: usize,
    mode: NoteDragMode,
}

#[derive(Clone, Copy, Debug)]
enum NoteDragMode {
    Create {
        anchor_beat: f32,
    },
    Move {
        beat_offset: f32,
        lane_offset: i32,
        length_beats: f32,
    },
    Resize {
        edge: ResizeEdge,
        fixed_beat: f32,
        edge_offset: f32,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResizeEdge {
    Start,
    End,
}

#[derive(Clone, Copy, Debug)]
struct NoteHit {
    note_index: usize,
    edge: Option<ResizeEdge>,
}

impl PianoRollView {
    pub fn new() -> Self {
        Self {
            active_tool: PianoRollTool::Draw,
            ghost_notes_enabled: false,
            ghost_track_indices: BTreeSet::new(),
            note_drag: None,
        }
    }

    pub fn show_window(
        &mut self,
        ctx: &egui::Context,
        open: &mut bool,
        active_track: &mut PianoRollTrackSnapshot,
        piano_roll_tracks: &[PianoRollTrackSnapshot],
    ) {
        if self
            .note_drag
            .is_some_and(|drag| drag.track_index != active_track.track_index)
        {
            self.note_drag = None;
        }
        let ghost_candidates: Vec<&PianoRollTrackSnapshot> =
            if active_track.kind == TrackKind::PianoRoll {
                piano_roll_tracks
                    .iter()
                    .filter(|track| track.track_index != active_track.track_index)
                    .collect()
            } else {
                Vec::new()
            };
        let valid_ghost_track_ids: BTreeSet<usize> = ghost_candidates
            .iter()
            .map(|track| track.track_index)
            .collect();
        self.ghost_track_indices
            .retain(|track_index| valid_ghost_track_ids.contains(track_index));
        if self.ghost_track_indices.is_empty() {
            self.ghost_notes_enabled = false;
        }

        egui::Window::new(active_track.name.as_str())
            .id(egui::Id::new("piano_roll_window"))
            .open(open)
            .default_size(egui::vec2(960.0, 560.0))
            .min_size(egui::vec2(560.0, 360.0))
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(theme::SURFACE_0)
                    .stroke(egui::Stroke::new(1.0, theme::BORDER)),
            )
            .show(ctx, |ui| {
                self.show_contents(ui, active_track, &ghost_candidates);
            });
    }

    fn show_contents(
        &mut self,
        ui: &mut egui::Ui,
        active_track: &mut PianoRollTrackSnapshot,
        ghost_candidates: &[&PianoRollTrackSnapshot],
    ) {
        let frame = egui::Frame::default()
            .fill(theme::SURFACE_0)
            .corner_radius(egui::CornerRadius::ZERO)
            .inner_margin(egui::Margin::ZERO);

        frame.show(ui, |ui| {
            self.show_header(ui, active_track);
            self.show_toolbar(ui, active_track, ghost_candidates);
            self.show_body(ui, active_track, ghost_candidates);
        });
    }

    fn show_header(&self, ui: &mut egui::Ui, active_track: &PianoRollTrackSnapshot) {
        let width = ui.available_width();
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(width, HEADER_HEIGHT), egui::Sense::click());
        let painter = ui.painter().clone();

        painter.rect_filled(rect, egui::CornerRadius::ZERO, theme::SURFACE_1);
        painter.line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            egui::Stroke::new(1.0, theme::BORDER),
        );

        painter.text(
            egui::pos2(rect.left() + 14.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            active_track.name.as_str(),
            egui::FontId::proportional(13.0),
            theme::TEXT,
        );

        let info_rect = egui::Rect::from_min_max(
            egui::pos2(rect.right() - 330.0, rect.top() + 6.0),
            egui::pos2(rect.right() - 12.0, rect.bottom() - 6.0),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(info_rect), |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                status_chip(ui, &active_track.lane_label);
                status_chip(
                    ui,
                    if active_track.kind == TrackKind::PianoRoll {
                        "Note Track"
                    } else {
                        "Audio Track"
                    },
                );
            });
        });
    }

    fn show_toolbar(
        &mut self,
        ui: &mut egui::Ui,
        active_track: &PianoRollTrackSnapshot,
        ghost_candidates: &[&PianoRollTrackSnapshot],
    ) {
        let width = ui.available_width();
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(width, TOOLBAR_HEIGHT), egui::Sense::click());
        let painter = ui.painter().clone();

        painter.rect_filled(rect, egui::CornerRadius::ZERO, theme::SURFACE_1);
        painter.line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            egui::Stroke::new(1.0, theme::BORDER),
        );

        let inner = rect.shrink2(egui::vec2(10.0, 6.0));
        ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;

                for tool in PianoRollTool::ALL {
                    let response = tool_button(
                        ui,
                        tool.icon(),
                        tool.label(),
                        self.active_tool == tool,
                        true,
                    );
                    if response.clicked() {
                        self.active_tool = tool;
                        self.note_drag = None;
                    }
                }

                ui.separator();

                let ghost_count = ghost_candidates
                    .iter()
                    .filter(|track| self.ghost_track_indices.contains(&track.track_index))
                    .count();
                let ghost_active = self.ghost_notes_enabled && ghost_count > 0;
                let ghost_label = if ghost_count > 0 {
                    format!("Ghost {}", ghost_count)
                } else {
                    "Ghost".to_owned()
                };
                let ghost_response = tool_button(
                    ui,
                    ToolIcon::Ghost,
                    &ghost_label,
                    ghost_active,
                    !ghost_candidates.is_empty(),
                );
                if ghost_response.clicked() && !ghost_candidates.is_empty() {
                    self.ghost_notes_enabled = !self.ghost_notes_enabled;
                    if self.ghost_notes_enabled && self.ghost_track_indices.is_empty() {
                        if let Some(first_track) = ghost_candidates.first() {
                            self.ghost_track_indices.insert(first_track.track_index);
                        }
                    }
                }
                ghost_response.context_menu(|ui| {
                    ui.set_min_width(220.0);
                    ui.label(
                        egui::RichText::new("Ghost Note Sources")
                            .strong()
                            .color(theme::TEXT),
                    );
                    ui.label(
                        egui::RichText::new(
                            "Show notes from other piano-roll tracks behind the active lane.",
                        )
                        .size(11.0)
                        .color(theme::TEXT_MUTED),
                    );
                    ui.separator();

                    if active_track.kind != TrackKind::PianoRoll {
                        ui.label(
                            egui::RichText::new(
                                "Select a piano-roll track in Playlist to configure ghosts.",
                            )
                            .size(11.0)
                            .color(theme::TEXT_MUTED),
                        );
                        return;
                    }

                    if ghost_candidates.is_empty() {
                        ui.label(
                            egui::RichText::new("No other piano-roll tracks are available.")
                                .size(11.0)
                                .color(theme::TEXT_MUTED),
                        );
                        return;
                    }

                    for track in ghost_candidates {
                        let mut selected = self.ghost_track_indices.contains(&track.track_index);
                        if ui.checkbox(&mut selected, track.name.as_str()).changed() {
                            if selected {
                                self.ghost_track_indices.insert(track.track_index);
                            } else {
                                self.ghost_track_indices.remove(&track.track_index);
                            }
                            self.ghost_notes_enabled = !self.ghost_track_indices.is_empty();
                        }
                    }
                });

                ui.separator();
                status_chip(ui, "Snap 1/16");
                status_chip(ui, "Velocity");
                status_chip(
                    ui,
                    if ghost_active {
                        "Ghosts On"
                    } else {
                        "Ghosts Off"
                    },
                );
            });
        });
    }

    fn show_body(
        &mut self,
        ui: &mut egui::Ui,
        active_track: &mut PianoRollTrackSnapshot,
        ghost_candidates: &[&PianoRollTrackSnapshot],
    ) {
        let available = ui.available_size_before_wrap();
        let (rect, _) = ui.allocate_exact_size(available, egui::Sense::click_and_drag());
        let painter = ui.painter().clone();

        painter.rect_filled(rect, egui::CornerRadius::ZERO, theme::SURFACE_0);

        if active_track.kind != TrackKind::PianoRoll {
            self.note_drag = None;
            draw_empty_state(&painter, rect, active_track.name.as_str());
            return;
        }

        let keyboard_rect = egui::Rect::from_min_max(
            rect.min,
            egui::pos2(rect.left() + KEYBOARD_WIDTH, rect.bottom()),
        );
        let grid_rect =
            egui::Rect::from_min_max(egui::pos2(keyboard_rect.right(), rect.top()), rect.max);

        draw_keyboard(&painter, keyboard_rect);
        draw_grid(&painter, grid_rect);

        if self.ghost_notes_enabled {
            let selected_ghost_tracks: Vec<&PianoRollTrackSnapshot> = ghost_candidates
                .iter()
                .copied()
                .filter(|track| self.ghost_track_indices.contains(&track.track_index))
                .collect();
            draw_ghost_tracks(&painter, grid_rect, &selected_ghost_tracks);
        }

        self.handle_note_interactions(ui, grid_rect, active_track);
        draw_active_notes(&painter, grid_rect, &active_track.notes);

        painter.line_segment(
            [
                egui::pos2(keyboard_rect.right(), rect.top()),
                egui::pos2(keyboard_rect.right(), rect.bottom()),
            ],
            egui::Stroke::new(1.0, theme::BORDER),
        );
    }

    fn handle_note_interactions(
        &mut self,
        ui: &mut egui::Ui,
        grid_rect: egui::Rect,
        active_track: &mut PianoRollTrackSnapshot,
    ) {
        if self.active_tool != PianoRollTool::Draw {
            self.note_drag = None;
            return;
        }

        let pointer_pos = ui.input(|input| input.pointer.interact_pos());
        let pointer_inside_grid = pointer_pos.is_some_and(|pos| grid_rect.contains(pos));
        let pointer_beat = pointer_pos.map(|pos| pos_to_beat(grid_rect, pos));
        let pointer_lane = pointer_pos.map(|pos| pos_to_lane(grid_rect, pos));

        if pointer_inside_grid {
            if let Some(pos) = pointer_pos {
                match note_hit_test(grid_rect, &active_track.notes, pos) {
                    Some(NoteHit { edge: Some(_), .. }) => ui.output_mut(|output| {
                        output.cursor_icon = egui::CursorIcon::ResizeHorizontal;
                    }),
                    Some(_) => ui.output_mut(|output| {
                        output.cursor_icon = egui::CursorIcon::Grab;
                    }),
                    None => ui.output_mut(|output| {
                        output.cursor_icon = egui::CursorIcon::Crosshair;
                    }),
                }
            }
        }

        let primary_pressed =
            ui.input(|input| input.pointer.button_pressed(egui::PointerButton::Primary));
        if primary_pressed && pointer_inside_grid {
            if let Some(pos) = pointer_pos {
                self.begin_primary_note_drag(
                    grid_rect,
                    pos,
                    active_track,
                    ui.input(|input| input.modifiers.alt),
                );
            }
        }

        let secondary_pressed =
            ui.input(|input| input.pointer.button_pressed(egui::PointerButton::Secondary));
        if secondary_pressed && pointer_inside_grid {
            if let Some(pos) = pointer_pos {
                if let Some(hit) = note_hit_test(grid_rect, &active_track.notes, pos) {
                    active_track.notes.remove(hit.note_index);
                    self.note_drag = None;
                }
            }
        }

        let primary_down =
            ui.input(|input| input.pointer.button_down(egui::PointerButton::Primary));
        if primary_down {
            if let (Some(pointer_beat), Some(pointer_lane)) = (pointer_beat, pointer_lane) {
                let free_drag = ui.input(|input| input.modifiers.alt);
                self.update_note_drag(
                    active_track,
                    pointer_beat,
                    pointer_lane,
                    grid_rect,
                    free_drag,
                );
            }
        } else if self.note_drag.is_some() {
            sort_notes(&mut active_track.notes);
            self.note_drag = None;
        }
    }

    fn begin_primary_note_drag(
        &mut self,
        grid_rect: egui::Rect,
        pointer_pos: egui::Pos2,
        active_track: &mut PianoRollTrackSnapshot,
        free_drag: bool,
    ) {
        let pointer_beat = pos_to_beat(grid_rect, pointer_pos);
        let pointer_lane = pos_to_lane(grid_rect, pointer_pos);

        if let Some(hit) = note_hit_test(grid_rect, &active_track.notes, pointer_pos) {
            let note = active_track.notes[hit.note_index].clone();
            let mode = match hit.edge {
                Some(edge) => {
                    let edge_beat = match edge {
                        ResizeEdge::Start => note.start_beat,
                        ResizeEdge::End => note.start_beat + note.length_beats,
                    };
                    NoteDragMode::Resize {
                        edge,
                        fixed_beat: match edge {
                            ResizeEdge::Start => note.start_beat + note.length_beats,
                            ResizeEdge::End => note.start_beat,
                        },
                        edge_offset: edge_beat - pointer_beat,
                    }
                }
                None => NoteDragMode::Move {
                    beat_offset: pointer_beat - note.start_beat,
                    lane_offset: pointer_lane as i32 - note.lane as i32,
                    length_beats: note.length_beats,
                },
            };

            self.note_drag = Some(NoteDrag {
                track_index: active_track.track_index,
                note_index: hit.note_index,
                mode,
            });
            self.update_note_drag(
                active_track,
                pointer_beat,
                pointer_lane,
                grid_rect,
                free_drag,
            );
            return;
        }

        let start_beat = quantize_beat(pointer_beat, free_drag);
        active_track.notes.push(PianoRollNoteData {
            lane: pointer_lane,
            start_beat,
            length_beats: MIN_NOTE_LENGTH,
            velocity: 0.88,
        });
        let note_index = active_track.notes.len() - 1;
        self.note_drag = Some(NoteDrag {
            track_index: active_track.track_index,
            note_index,
            mode: NoteDragMode::Create {
                anchor_beat: start_beat,
            },
        });
        self.update_note_drag(
            active_track,
            pointer_beat,
            pointer_lane,
            grid_rect,
            free_drag,
        );
    }

    fn update_note_drag(
        &mut self,
        active_track: &mut PianoRollTrackSnapshot,
        pointer_beat: f32,
        pointer_lane: usize,
        grid_rect: egui::Rect,
        free_drag: bool,
    ) {
        let Some(drag) = self.note_drag else {
            return;
        };
        if drag.track_index != active_track.track_index {
            self.note_drag = None;
            return;
        }

        let grid_beats = grid_total_beats(grid_rect);
        let Some(note) = active_track.notes.get_mut(drag.note_index) else {
            self.note_drag = None;
            return;
        };

        match drag.mode {
            NoteDragMode::Create { anchor_beat } => {
                let dragged_beat = quantize_beat(pointer_beat, free_drag);
                let start = anchor_beat.min(dragged_beat);
                let end = anchor_beat.max(dragged_beat).max(start + MIN_NOTE_LENGTH);
                note.start_beat = start.clamp(0.0, (grid_beats - MIN_NOTE_LENGTH).max(0.0));
                note.length_beats =
                    clamp_note_length(end - note.start_beat, note.start_beat, grid_beats);
                note.lane = pointer_lane;
            }
            NoteDragMode::Move {
                beat_offset,
                lane_offset,
                length_beats,
            } => {
                let start = quantize_beat(pointer_beat - beat_offset, free_drag);
                note.start_beat = clamp_start_beat(start, length_beats, grid_beats);
                note.length_beats = clamp_note_length(length_beats, note.start_beat, grid_beats);
                note.lane = clamp_lane(pointer_lane as i32 - lane_offset);
            }
            NoteDragMode::Resize {
                edge,
                fixed_beat,
                edge_offset,
            } => {
                let edge_beat = quantize_beat(pointer_beat + edge_offset, free_drag);
                match edge {
                    ResizeEdge::Start => {
                        let start = edge_beat.clamp(0.0, (fixed_beat - MIN_NOTE_LENGTH).max(0.0));
                        note.start_beat = start;
                        note.length_beats =
                            clamp_note_length(fixed_beat - start, start, grid_beats);
                    }
                    ResizeEdge::End => {
                        note.start_beat = fixed_beat;
                        note.length_beats =
                            clamp_note_length(edge_beat - fixed_beat, fixed_beat, grid_beats);
                    }
                }
            }
        }
    }
}

impl Default for PianoRollView {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PianoRollTool {
    Select,
    Draw,
    Split,
    Mute,
}

impl PianoRollTool {
    const ALL: [Self; 4] = [Self::Select, Self::Draw, Self::Split, Self::Mute];

    const fn label(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::Draw => "Brush",
            Self::Split => "Split",
            Self::Mute => "Mute",
        }
    }

    const fn icon(self) -> ToolIcon {
        match self {
            Self::Select => ToolIcon::Select,
            Self::Draw => ToolIcon::Draw,
            Self::Split => ToolIcon::Split,
            Self::Mute => ToolIcon::Mute,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ToolIcon {
    Select,
    Draw,
    Split,
    Mute,
    Ghost,
}

fn draw_keyboard(painter: &egui::Painter, rect: egui::Rect) {
    painter.rect_filled(rect, egui::CornerRadius::ZERO, theme::SURFACE_1);

    for lane in 0..NOTE_ROWS {
        let top = rect.top() + lane as f32 * ROW_HEIGHT;
        if top >= rect.bottom() {
            break;
        }

        let row_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), top),
            egui::pos2(rect.right(), (top + ROW_HEIGHT).min(rect.bottom())),
        );
        let midi_note = TOP_MIDI_NOTE - lane as i32;
        let fill = if is_black_key(midi_note) {
            theme::SURFACE_2
        } else if lane % 2 == 0 {
            theme::SURFACE_1
        } else {
            theme::SURFACE_0
        };

        painter.rect_filled(row_rect, egui::CornerRadius::ZERO, fill);
        painter.line_segment(
            [row_rect.left_bottom(), row_rect.right_bottom()],
            egui::Stroke::new(1.0, theme::GRID_ROW),
        );
        painter.text(
            egui::pos2(row_rect.left() + 12.0, row_rect.center().y),
            egui::Align2::LEFT_CENTER,
            note_label(midi_note),
            egui::FontId::proportional(11.0),
            theme::TEXT_MUTED,
        );
    }
}

fn draw_grid(painter: &egui::Painter, rect: egui::Rect) {
    painter.rect_filled(rect, egui::CornerRadius::ZERO, theme::SURFACE_0);

    for lane in 0..NOTE_ROWS {
        let top = rect.top() + lane as f32 * ROW_HEIGHT;
        if top >= rect.bottom() {
            break;
        }

        let row_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left(), top),
            egui::pos2(rect.right(), (top + ROW_HEIGHT).min(rect.bottom())),
        );
        let midi_note = TOP_MIDI_NOTE - lane as i32;
        let fill = if is_black_key(midi_note) {
            theme::GRID_MINOR
        } else if lane % 2 == 0 {
            theme::SURFACE_0
        } else {
            theme::SURFACE_1
        };
        painter.rect_filled(row_rect, egui::CornerRadius::ZERO, fill);
        painter.line_segment(
            [row_rect.left_bottom(), row_rect.right_bottom()],
            egui::Stroke::new(1.0, theme::GRID_ROW),
        );
    }

    let mut beat = 0;
    let mut x = rect.left();
    while x <= rect.right() {
        let color = if beat % 4 == 0 {
            theme::GRID_MAJOR
        } else {
            theme::GRID_ROW
        };
        let width = if beat % 4 == 0 { 1.5 } else { 1.0 };
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(width, color),
        );

        if x + BEAT_WIDTH < rect.right() {
            painter.text(
                egui::pos2(x + 6.0, rect.top() + 10.0),
                egui::Align2::LEFT_CENTER,
                format!("{}", beat + 1),
                egui::FontId::proportional(10.0),
                theme::TEXT_MUTED,
            );
        }

        x += BEAT_WIDTH;
        beat += 1;
    }
}

fn draw_ghost_tracks(
    painter: &egui::Painter,
    rect: egui::Rect,
    tracks: &[&PianoRollTrackSnapshot],
) {
    for (slot, track) in tracks.iter().enumerate() {
        let (fill, stroke) = ghost_palette(slot);
        draw_notes(painter, rect, &track.notes, fill, stroke, true);
    }
}

fn draw_active_notes(painter: &egui::Painter, rect: egui::Rect, notes: &[PianoRollNoteData]) {
    draw_notes(
        painter,
        rect,
        notes,
        theme::ACCENT,
        theme::ACCENT_HOVER,
        false,
    );
}

fn draw_notes(
    painter: &egui::Painter,
    rect: egui::Rect,
    notes: &[PianoRollNoteData],
    fill_color: egui::Color32,
    stroke_color: egui::Color32,
    ghost: bool,
) {
    for note in notes {
        let note_rect = note_rect(rect, note);

        if note_rect.top() >= rect.bottom() || note_rect.left() >= rect.right() {
            continue;
        }

        let alpha = if ghost {
            (22.0 + note.velocity * 18.0).round() as u8
        } else {
            (160.0 + note.velocity * 70.0).round() as u8
        };
        let fill = egui::Color32::from_rgba_unmultiplied(
            fill_color.r(),
            fill_color.g(),
            fill_color.b(),
            alpha,
        );
        let stroke = egui::Color32::from_rgba_unmultiplied(
            stroke_color.r(),
            stroke_color.g(),
            stroke_color.b(),
            if ghost { 44 } else { 220 },
        );

        painter.rect_filled(note_rect, 4.0, fill);
        painter.rect_stroke(
            note_rect,
            4.0,
            egui::Stroke::new(if ghost { 1.0 } else { 1.2 }, stroke),
            egui::StrokeKind::Inside,
        );
        if !ghost {
            painter.line_segment(
                [
                    egui::pos2(note_rect.left() + 10.0, note_rect.bottom() - 4.0),
                    egui::pos2(note_rect.right() - 8.0, note_rect.bottom() - 4.0),
                ],
                egui::Stroke::new(1.0, theme::TOOL_ACTIVE_WASH),
            );
        }
    }
}

fn draw_empty_state(painter: &egui::Painter, rect: egui::Rect, active_track_name: &str) {
    let card_rect = rect.shrink2(egui::vec2(80.0, 64.0));
    painter.rect_filled(card_rect, 8.0, theme::SURFACE_1);
    painter.rect_stroke(
        card_rect,
        8.0,
        egui::Stroke::new(1.0, theme::BORDER),
        egui::StrokeKind::Inside,
    );
    painter.text(
        egui::pos2(card_rect.center().x, card_rect.center().y - 12.0),
        egui::Align2::CENTER_CENTER,
        format!("{active_track_name} is an audio track"),
        egui::FontId::proportional(15.0),
        theme::TEXT,
    );
    painter.text(
        egui::pos2(card_rect.center().x, card_rect.center().y + 14.0),
        egui::Align2::CENTER_CENTER,
        "Select a note-based playlist track to edit notes or enable ghost sources.",
        egui::FontId::proportional(11.5),
        theme::TEXT_MUTED,
    );
}

fn tool_button(
    ui: &mut egui::Ui,
    icon: ToolIcon,
    label: &str,
    active: bool,
    enabled: bool,
) -> egui::Response {
    let mut style = ui.style().as_ref().clone();
    style.visuals.widgets.noninteractive.fg_stroke.color = theme::GRID_MAJOR;
    style.visuals.widgets.inactive.fg_stroke.color = theme::TEXT;
    style.visuals.widgets.hovered.fg_stroke.color = theme::ACCENT;
    style.visuals.widgets.active.fg_stroke.color = theme::ACCENT;
    style.visuals.widgets.open.fg_stroke.color = theme::ACCENT;
    style.visuals.selection.stroke.color = theme::ACCENT;

    let image = egui::Image::new(icon_source(icon))
        .fit_to_exact_size(egui::vec2(14.0, 14.0))
        .tint(egui::Color32::WHITE);
    let button = egui::Button::image(image)
        .min_size(egui::vec2(28.0, 24.0))
        .fill(if active {
            theme::TOOL_ACTIVE_WASH
        } else {
            theme::SURFACE_0
        })
        .stroke(egui::Stroke::new(
            1.0,
            if active {
                theme::ACCENT
            } else if enabled {
                theme::BORDER
            } else {
                theme::GRID_ROW
            },
        ))
        .corner_radius(3.0)
        .image_tint_follows_text_color(true);
    ui.scope(|ui| {
        ui.set_style(style);
        ui.add_enabled(enabled, button)
    })
    .inner
    .on_hover_text(label)
}

fn status_chip(ui: &mut egui::Ui, label: &str) {
    let text = egui::RichText::new(label)
        .size(10.5)
        .color(theme::TEXT_MUTED);
    let button = egui::Button::new(text)
        .min_size(egui::vec2(0.0, 24.0))
        .fill(theme::SURFACE_0)
        .stroke(egui::Stroke::new(1.0, theme::BORDER))
        .corner_radius(3.0);
    ui.add(button);
}

fn ghost_palette(slot: usize) -> (egui::Color32, egui::Color32) {
    const COLORS: [(egui::Color32, egui::Color32); 4] = [
        (
            egui::Color32::from_rgb(255, 196, 107),
            egui::Color32::from_rgb(255, 213, 145),
        ),
        (
            egui::Color32::from_rgb(135, 214, 168),
            egui::Color32::from_rgb(169, 227, 193),
        ),
        (
            egui::Color32::from_rgb(244, 137, 165),
            egui::Color32::from_rgb(250, 172, 191),
        ),
        (
            egui::Color32::from_rgb(190, 165, 255),
            egui::Color32::from_rgb(209, 191, 255),
        ),
    ];
    COLORS[slot % COLORS.len()]
}

fn icon_source(icon: ToolIcon) -> egui::ImageSource<'static> {
    match icon {
        ToolIcon::Select => egui::include_image!("../assets/piano_roll_tools/select.svg"),
        ToolIcon::Draw => egui::include_image!("../assets/piano_roll_tools/draw.svg"),
        ToolIcon::Split => egui::include_image!("../assets/piano_roll_tools/split.svg"),
        ToolIcon::Mute => egui::include_image!("../assets/piano_roll_tools/mute.svg"),
        ToolIcon::Ghost => egui::include_image!("../assets/piano_roll_tools/ghost.svg"),
    }
}

fn note_hit_test(
    rect: egui::Rect,
    notes: &[PianoRollNoteData],
    pos: egui::Pos2,
) -> Option<NoteHit> {
    for (note_index, note) in notes.iter().enumerate().rev() {
        let note_rect = note_rect(rect, note);
        if !note_rect.contains(pos) {
            continue;
        }

        let handle_width = RESIZE_HANDLE_WIDTH.min(note_rect.width() * 0.5);
        let edge = if pos.x <= note_rect.left() + handle_width {
            Some(ResizeEdge::Start)
        } else if pos.x >= note_rect.right() - handle_width {
            Some(ResizeEdge::End)
        } else {
            None
        };
        return Some(NoteHit { note_index, edge });
    }

    None
}

fn note_rect(rect: egui::Rect, note: &PianoRollNoteData) -> egui::Rect {
    let top = rect.top() + note.lane as f32 * ROW_HEIGHT + 3.0;
    let bottom = (top + ROW_HEIGHT - 6.0).min(rect.bottom() - 2.0);
    let left = rect.left() + note.start_beat * BEAT_WIDTH;
    let right = (left + note.length_beats * BEAT_WIDTH).min(rect.right() - 3.0);
    egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom))
}

fn pos_to_lane(rect: egui::Rect, pos: egui::Pos2) -> usize {
    ((pos.y - rect.top()) / ROW_HEIGHT)
        .floor()
        .clamp(0.0, (NOTE_ROWS - 1) as f32) as usize
}

fn pos_to_beat(rect: egui::Rect, pos: egui::Pos2) -> f32 {
    ((pos.x - rect.left()) / BEAT_WIDTH).clamp(0.0, grid_total_beats(rect))
}

fn grid_total_beats(rect: egui::Rect) -> f32 {
    (rect.width() / BEAT_WIDTH).max(MIN_NOTE_LENGTH)
}

fn quantize_beat(beat: f32, free_drag: bool) -> f32 {
    if free_drag {
        beat.max(0.0)
    } else {
        (beat / SNAP_BEATS).round() * SNAP_BEATS
    }
}

fn clamp_lane(lane: i32) -> usize {
    lane.clamp(0, NOTE_ROWS as i32 - 1) as usize
}

fn clamp_start_beat(start: f32, length: f32, grid_beats: f32) -> f32 {
    start.clamp(0.0, (grid_beats - length.max(MIN_NOTE_LENGTH)).max(0.0))
}

fn clamp_note_length(length: f32, start: f32, grid_beats: f32) -> f32 {
    length
        .max(MIN_NOTE_LENGTH)
        .min((grid_beats - start).max(MIN_NOTE_LENGTH))
}

fn sort_notes(notes: &mut [PianoRollNoteData]) {
    notes.sort_by(|left, right| {
        left.start_beat
            .total_cmp(&right.start_beat)
            .then(left.lane.cmp(&right.lane))
            .then(left.length_beats.total_cmp(&right.length_beats))
    });
}

fn note_label(midi_note: i32) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let note_index = midi_note.rem_euclid(12) as usize;
    let octave = midi_note.div_euclid(12) - 1;
    format!("{}{}", NAMES[note_index], octave)
}

fn is_black_key(midi_note: i32) -> bool {
    matches!(midi_note.rem_euclid(12), 1 | 3 | 6 | 8 | 10)
}
