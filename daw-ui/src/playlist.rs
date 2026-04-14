use eframe::egui;

use crate::theme;

const TOOLBAR_HEIGHT: f32 = 38.0;
const TIMELINE_HEIGHT: f32 = 34.0;
const TRACK_HEADER_WIDTH: f32 = 164.0;
const TRACK_HEIGHT: f32 = 72.0;
const TRACK_GAP: f32 = 6.0;
const SNAP_BEATS: f32 = 0.25;
const MIN_CLIP_LENGTH: f32 = SNAP_BEATS;
const RESIZE_HANDLE_WIDTH: f32 = 8.0;
const CLIP_VERTICAL_PADDING: f32 = 12.0;
const DEFAULT_VISIBLE_BARS: f32 = 8.0;
const MIN_VISIBLE_BARS: f32 = 1.0;
const MAX_VISIBLE_BARS: f32 = 32.0;
const SCROLL_SENSITIVITY: f32 = 0.35;
const ZOOM_SENSITIVITY: f32 = 0.01;

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
    selected_clip: Option<ClipSelection>,
    clip_drag: Option<ClipDrag>,
    next_clip_id: usize,
    viewport: PlaylistViewport,
    tracks: Vec<PlaylistTrack>,
}

#[derive(Clone, Copy, Debug)]
struct ClipSelection {
    track_index: usize,
    clip_id: usize,
}

#[derive(Clone, Copy, Debug)]
struct ClipDrag {
    track_index: usize,
    clip_id: usize,
    mode: ClipDragMode,
}

#[derive(Clone, Copy, Debug)]
enum ClipDragMode {
    Create {
        anchor_beat: f32,
    },
    Move {
        beat_offset: f32,
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
struct ClipHit {
    track_index: usize,
    clip_id: usize,
    edge: Option<ResizeEdge>,
}

#[derive(Clone, Copy, Debug)]
struct PlaylistViewport {
    visible_bars: f32,
    vertical_scroll: f32,
}

impl PlaylistView {
    pub fn new() -> Self {
        let tracks = sample_tracks();
        let next_clip_id = tracks
            .iter()
            .flat_map(|track| track.clips.iter().map(|clip| clip.id))
            .max()
            .map_or(0, |id| id + 1);
        Self {
            active_tool: PlaylistTool::Select,
            selected_track: 0,
            selected_clip: None,
            clip_drag: None,
            next_clip_id,
            viewport: PlaylistViewport::new(),
            tracks,
        }
    }

    pub fn active_track_name(&self) -> &str {
        self.tracks
            .get(self.selected_track)
            .map(|track| track.name.as_str())
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

    pub fn show(&mut self, ui: &mut egui::Ui, interactions_enabled: bool) {
        let rect = ui.max_rect();
        let frame = egui::Frame::default()
            .fill(theme::SURFACE_0)
            .corner_radius(egui::CornerRadius::ZERO)
            .inner_margin(egui::Margin::ZERO);

        frame.show(ui, |ui| {
            self.show_toolbar(ui);
            self.show_body(ui, interactions_enabled);
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

        let selected_clip_summary = self.selected_clip_summary();
        let toolbar_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left() + TRACK_HEADER_WIDTH + 8.0, rect.top() + 6.0),
            egui::pos2(rect.right() - 8.0, rect.bottom() - 6.0),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(toolbar_rect), |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;

                for tool in PlaylistTool::ALL {
                    let response = tool_button(
                        ui,
                        tool.icon(),
                        tool.label(),
                        self.active_tool == tool,
                        true,
                    );
                    if response.clicked() {
                        self.active_tool = tool;
                        self.clip_drag = None;
                    }
                }

                ui.separator();
                status_chip(ui, "Snap 1/16");
                status_chip(ui, &format!("Bars {:.1}", self.viewport.visible_bars));
                status_chip(
                    ui,
                    if ui.input(|input| input.modifiers.alt) {
                        "Free Drag"
                    } else {
                        "Quantized"
                    },
                );
                if let Some(summary) = &selected_clip_summary {
                    status_chip(ui, summary);
                }
            });
        });
    }

    fn show_body(&mut self, ui: &mut egui::Ui, interactions_enabled: bool) {
        let available = ui.available_size_before_wrap();
        let (rect, response) = ui.allocate_exact_size(available, egui::Sense::click_and_drag());
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
        let content_height = playlist_content_height(self.tracks.len());
        self.viewport.apply_input(
            ui,
            interactions_enabled && response.hovered(),
            track_area_rect.height(),
            content_height,
        );

        if interactions_enabled {
            self.handle_clip_interactions(ui, sidebar_rect, grid_rect, self.viewport);
        }

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

        let sidebar_painter = painter.with_clip_rect(sidebar_rect);
        let grid_painter = painter.with_clip_rect(grid_rect);

        draw_timeline(&painter, timeline_rect, TRACK_HEADER_WIDTH, self.viewport);
        draw_grid(&grid_painter, grid_rect, self.tracks.len(), self.viewport);
        draw_tracks(
            &sidebar_painter,
            &grid_painter,
            sidebar_rect,
            grid_rect,
            &self.tracks,
            self.selected_track,
            self.selected_clip,
            self.viewport,
        );
    }

    fn handle_clip_interactions(
        &mut self,
        ui: &mut egui::Ui,
        sidebar_rect: egui::Rect,
        grid_rect: egui::Rect,
        viewport: PlaylistViewport,
    ) {
        let pointer_pos = ui.input(|input| input.pointer.interact_pos());
        let pointer_in_sidebar = pointer_pos.is_some_and(|pos| sidebar_rect.contains(pos));
        let pointer_in_grid = pointer_pos.is_some_and(|pos| grid_rect.contains(pos));

        if pointer_in_grid {
            if let Some(pos) = pointer_pos {
                let hit = clip_hit_test(grid_rect, &self.tracks, pos, viewport);
                match self.active_tool {
                    PlaylistTool::Select | PlaylistTool::Draw => match hit {
                        Some(ClipHit { edge: Some(_), .. }) => ui.output_mut(|output| {
                            output.cursor_icon = egui::CursorIcon::ResizeHorizontal;
                        }),
                        Some(_) => ui.output_mut(|output| {
                            output.cursor_icon = egui::CursorIcon::Grab;
                        }),
                        None if self.active_tool == PlaylistTool::Draw => ui.output_mut(|output| {
                            output.cursor_icon = egui::CursorIcon::Crosshair;
                        }),
                        None => {}
                    },
                    PlaylistTool::Split => {
                        if hit.is_some() {
                            ui.output_mut(|output| {
                                output.cursor_icon = egui::CursorIcon::Crosshair;
                            });
                        }
                    }
                    PlaylistTool::Duplicate | PlaylistTool::Mute => {
                        if hit.is_some() {
                            ui.output_mut(|output| {
                                output.cursor_icon = egui::CursorIcon::PointingHand;
                            });
                        }
                    }
                }
            }
        }

        let free_drag = ui.input(|input| input.modifiers.alt);
        let primary_pressed =
            ui.input(|input| input.pointer.button_pressed(egui::PointerButton::Primary));
        if primary_pressed {
            if let Some(pos) = pointer_pos {
                if pointer_in_sidebar {
                    if let Some(track_index) =
                        pos_to_track(sidebar_rect, pos, self.tracks.len(), viewport)
                    {
                        self.selected_track = track_index;
                        self.selected_clip = None;
                        self.clip_drag = None;
                    }
                } else if pointer_in_grid {
                    self.handle_grid_primary_pressed(grid_rect, pos, viewport, free_drag);
                } else {
                    self.clip_drag = None;
                }
            }
        }

        let secondary_pressed =
            ui.input(|input| input.pointer.button_pressed(egui::PointerButton::Secondary));
        if secondary_pressed && pointer_in_grid {
            if let Some(pos) = pointer_pos {
                if let Some(hit) = clip_hit_test(grid_rect, &self.tracks, pos, viewport) {
                    self.remove_clip(hit.track_index, hit.clip_id);
                }
            }
        }

        let primary_down =
            ui.input(|input| input.pointer.button_down(egui::PointerButton::Primary));
        if primary_down {
            if let Some(pos) = pointer_pos {
                let pointer_beat = pos_to_beat(grid_rect, pos, viewport);
                self.update_clip_drag(pointer_beat, grid_rect, viewport, free_drag);
            }
        } else if self.clip_drag.is_some() {
            self.finish_clip_drag();
        }
    }

    fn handle_grid_primary_pressed(
        &mut self,
        grid_rect: egui::Rect,
        pointer_pos: egui::Pos2,
        viewport: PlaylistViewport,
        free_drag: bool,
    ) {
        let pointer_beat = pos_to_beat(grid_rect, pointer_pos, viewport);
        let pointer_track = pos_to_track(grid_rect, pointer_pos, self.tracks.len(), viewport);
        let hit = clip_hit_test(grid_rect, &self.tracks, pointer_pos, viewport);

        match self.active_tool {
            PlaylistTool::Select => {
                if let Some(hit) = hit {
                    self.selected_track = hit.track_index;
                    self.selected_clip = Some(ClipSelection {
                        track_index: hit.track_index,
                        clip_id: hit.clip_id,
                    });
                    self.begin_clip_drag_from_hit(
                        hit,
                        pointer_beat,
                        free_drag,
                        grid_rect,
                        viewport,
                    );
                } else if let Some(track_index) = pointer_track {
                    self.selected_track = track_index;
                    self.selected_clip = None;
                    self.clip_drag = None;
                }
            }
            PlaylistTool::Draw => {
                if let Some(hit) = hit {
                    self.selected_track = hit.track_index;
                    self.selected_clip = Some(ClipSelection {
                        track_index: hit.track_index,
                        clip_id: hit.clip_id,
                    });
                    self.begin_clip_drag_from_hit(
                        hit,
                        pointer_beat,
                        free_drag,
                        grid_rect,
                        viewport,
                    );
                } else if let Some(track_index) = pointer_track {
                    self.selected_track = track_index;
                    let clip_id =
                        self.create_clip(track_index, quantize_beat(pointer_beat, free_drag));
                    self.selected_clip = Some(ClipSelection {
                        track_index,
                        clip_id,
                    });
                    self.clip_drag = Some(ClipDrag {
                        track_index,
                        clip_id,
                        mode: ClipDragMode::Create {
                            anchor_beat: quantize_beat(pointer_beat, free_drag),
                        },
                    });
                    self.update_clip_drag(pointer_beat, grid_rect, viewport, free_drag);
                }
            }
            PlaylistTool::Split => {
                self.clip_drag = None;
                if let Some(hit) = hit {
                    self.selected_track = hit.track_index;
                    self.selected_clip = Some(ClipSelection {
                        track_index: hit.track_index,
                        clip_id: hit.clip_id,
                    });
                    self.split_clip(
                        hit.track_index,
                        hit.clip_id,
                        quantize_beat(pointer_beat, free_drag),
                    );
                } else if let Some(track_index) = pointer_track {
                    self.selected_track = track_index;
                    self.selected_clip = None;
                }
            }
            PlaylistTool::Duplicate => {
                self.clip_drag = None;
                if let Some(hit) = hit {
                    self.selected_track = hit.track_index;
                    self.selected_clip = self.duplicate_clip(hit.track_index, hit.clip_id);
                } else if let Some(track_index) = pointer_track {
                    self.selected_track = track_index;
                    self.selected_clip = None;
                }
            }
            PlaylistTool::Mute => {
                self.clip_drag = None;
                if let Some(hit) = hit {
                    self.selected_track = hit.track_index;
                    self.toggle_clip_mute(hit.track_index, hit.clip_id);
                    self.selected_clip = Some(ClipSelection {
                        track_index: hit.track_index,
                        clip_id: hit.clip_id,
                    });
                } else if let Some(track_index) = pointer_track {
                    self.selected_track = track_index;
                    self.selected_clip = None;
                }
            }
        }
    }

    fn begin_clip_drag_from_hit(
        &mut self,
        hit: ClipHit,
        pointer_beat: f32,
        free_drag: bool,
        grid_rect: egui::Rect,
        viewport: PlaylistViewport,
    ) {
        let Some(clip) = self.clip(hit.track_index, hit.clip_id).cloned() else {
            self.clip_drag = None;
            return;
        };

        let mode = match hit.edge {
            Some(edge) => {
                let edge_beat = match edge {
                    ResizeEdge::Start => clip.start_beat,
                    ResizeEdge::End => clip.start_beat + clip.length_beats,
                };
                ClipDragMode::Resize {
                    edge,
                    fixed_beat: match edge {
                        ResizeEdge::Start => clip.start_beat + clip.length_beats,
                        ResizeEdge::End => clip.start_beat,
                    },
                    edge_offset: edge_beat - pointer_beat,
                }
            }
            None => ClipDragMode::Move {
                beat_offset: pointer_beat - clip.start_beat,
                length_beats: clip.length_beats,
            },
        };

        self.clip_drag = Some(ClipDrag {
            track_index: hit.track_index,
            clip_id: hit.clip_id,
            mode,
        });
        self.update_clip_drag(pointer_beat, grid_rect, viewport, free_drag);
    }

    fn update_clip_drag(
        &mut self,
        pointer_beat: f32,
        _grid_rect: egui::Rect,
        viewport: PlaylistViewport,
        free_drag: bool,
    ) {
        let Some(drag) = self.clip_drag else {
            return;
        };
        let grid_beats = grid_total_beats(viewport);
        let Some(clip) = self.clip_mut(drag.track_index, drag.clip_id) else {
            self.clip_drag = None;
            return;
        };

        match drag.mode {
            ClipDragMode::Create { anchor_beat } => {
                let dragged_beat = quantize_beat(pointer_beat, free_drag);
                let start = anchor_beat.min(dragged_beat);
                let end = anchor_beat.max(dragged_beat).max(start + MIN_CLIP_LENGTH);
                clip.start_beat = start.clamp(0.0, (grid_beats - MIN_CLIP_LENGTH).max(0.0));
                clip.length_beats =
                    clamp_clip_length(end - clip.start_beat, clip.start_beat, grid_beats);
            }
            ClipDragMode::Move {
                beat_offset,
                length_beats,
            } => {
                let start = quantize_beat(pointer_beat - beat_offset, free_drag);
                clip.start_beat = clamp_start_beat(start, length_beats, grid_beats);
                clip.length_beats = clamp_clip_length(length_beats, clip.start_beat, grid_beats);
            }
            ClipDragMode::Resize {
                edge,
                fixed_beat,
                edge_offset,
            } => {
                let edge_beat = quantize_beat(pointer_beat + edge_offset, free_drag);
                match edge {
                    ResizeEdge::Start => {
                        let start = edge_beat.clamp(0.0, (fixed_beat - MIN_CLIP_LENGTH).max(0.0));
                        clip.start_beat = start;
                        clip.length_beats =
                            clamp_clip_length(fixed_beat - start, start, grid_beats);
                    }
                    ResizeEdge::End => {
                        clip.start_beat = fixed_beat;
                        clip.length_beats =
                            clamp_clip_length(edge_beat - fixed_beat, fixed_beat, grid_beats);
                    }
                }
            }
        }
    }

    fn finish_clip_drag(&mut self) {
        if let Some(drag) = self.clip_drag.take() {
            if let Some(track) = self.tracks.get_mut(drag.track_index) {
                sort_clips(&mut track.clips);
            }
        }
    }

    fn create_clip(&mut self, track_index: usize, start_beat: f32) -> usize {
        let clip_id = self.next_clip_id;
        self.next_clip_id += 1;
        let clip = PlaylistClipData {
            id: clip_id,
            label: default_clip_label(&self.tracks[track_index]),
            start_beat,
            length_beats: MIN_CLIP_LENGTH,
            muted: false,
        };
        self.tracks[track_index].clips.push(clip);
        clip_id
    }

    fn split_clip(&mut self, track_index: usize, clip_id: usize, split_beat: f32) {
        let Some(track) = self.tracks.get_mut(track_index) else {
            return;
        };
        let Some(index) = track.clips.iter().position(|clip| clip.id == clip_id) else {
            return;
        };

        let clip = track.clips[index].clone();
        let clip_end = clip.start_beat + clip.length_beats;
        if split_beat <= clip.start_beat + MIN_CLIP_LENGTH
            || split_beat >= clip_end - MIN_CLIP_LENGTH
        {
            return;
        }

        track.clips[index].length_beats = split_beat - clip.start_beat;
        let new_clip_id = self.next_clip_id;
        self.next_clip_id += 1;
        let mut split_clip = clip;
        split_clip.id = new_clip_id;
        split_clip.start_beat = split_beat;
        split_clip.length_beats = clip_end - split_beat;
        track.clips.push(split_clip);
        sort_clips(&mut track.clips);
        self.selected_clip = Some(ClipSelection {
            track_index,
            clip_id: new_clip_id,
        });
    }

    fn duplicate_clip(&mut self, track_index: usize, clip_id: usize) -> Option<ClipSelection> {
        let source_clip = self.clip(track_index, clip_id)?.clone();
        let new_clip_id = self.next_clip_id;
        self.next_clip_id += 1;

        let mut duplicate = source_clip;
        duplicate.id = new_clip_id;
        duplicate.start_beat += duplicate.length_beats;

        let track = self.tracks.get_mut(track_index)?;
        track.clips.push(duplicate);
        sort_clips(&mut track.clips);

        Some(ClipSelection {
            track_index,
            clip_id: new_clip_id,
        })
    }

    fn toggle_clip_mute(&mut self, track_index: usize, clip_id: usize) {
        if let Some(clip) = self.clip_mut(track_index, clip_id) {
            clip.muted = !clip.muted;
        }
    }

    fn remove_clip(&mut self, track_index: usize, clip_id: usize) {
        let Some(track) = self.tracks.get_mut(track_index) else {
            return;
        };
        let Some(index) = track.clips.iter().position(|clip| clip.id == clip_id) else {
            return;
        };
        track.clips.remove(index);

        if self.selected_clip.is_some_and(|selection| {
            selection.track_index == track_index && selection.clip_id == clip_id
        }) {
            self.selected_clip = None;
        }
        if self
            .clip_drag
            .is_some_and(|drag| drag.track_index == track_index && drag.clip_id == clip_id)
        {
            self.clip_drag = None;
        }
    }

    fn clip(&self, track_index: usize, clip_id: usize) -> Option<&PlaylistClipData> {
        self.tracks
            .get(track_index)?
            .clips
            .iter()
            .find(|clip| clip.id == clip_id)
    }

    fn clip_mut(&mut self, track_index: usize, clip_id: usize) -> Option<&mut PlaylistClipData> {
        self.tracks
            .get_mut(track_index)?
            .clips
            .iter_mut()
            .find(|clip| clip.id == clip_id)
    }

    fn selected_clip_summary(&self) -> Option<String> {
        let selection = self.selected_clip?;
        let clip = self.clip(selection.track_index, selection.clip_id)?;
        let bars = clip.length_beats / 4.0;
        Some(if clip.muted {
            format!("{} · {:.1} bars · Muted", clip.label, bars)
        } else {
            format!("{} · {:.1} bars", clip.label, bars)
        })
    }
}

impl Default for PlaylistView {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaylistViewport {
    fn new() -> Self {
        Self {
            visible_bars: DEFAULT_VISIBLE_BARS,
            vertical_scroll: 0.0,
        }
    }

    fn apply_input(
        &mut self,
        ui: &egui::Ui,
        hovered: bool,
        viewport_height: f32,
        content_height: f32,
    ) {
        self.clamp_scroll(viewport_height, content_height);
        if !hovered {
            return;
        }

        let (scroll_y, ctrl_held) = ui.input(|input| {
            let delta = if input.smooth_scroll_delta.y.abs() > f32::EPSILON {
                input.smooth_scroll_delta.y
            } else {
                input.raw_scroll_delta.y
            };
            (delta, input.modifiers.ctrl)
        });
        if scroll_y.abs() <= f32::EPSILON {
            return;
        }

        if ctrl_held {
            self.visible_bars = zoom_visible_bars(self.visible_bars, scroll_y);
        } else {
            self.vertical_scroll -= scroll_y * SCROLL_SENSITIVITY;
            self.clamp_scroll(viewport_height, content_height);
        }
    }

    fn clamp_scroll(&mut self, viewport_height: f32, content_height: f32) {
        let max_scroll = (content_height - viewport_height).max(0.0);
        self.vertical_scroll = self.vertical_scroll.clamp(0.0, max_scroll);
    }
}

#[derive(Clone, Debug)]
struct PlaylistTrack {
    name: String,
    lane_label: String,
    kind: TrackKind,
    notes: Vec<PianoRollNoteData>,
    clips: Vec<PlaylistClipData>,
}

impl PlaylistTrack {
    fn piano_roll(
        name: impl Into<String>,
        lane_label: impl Into<String>,
        notes: Vec<PianoRollNoteData>,
        clips: Vec<PlaylistClipData>,
    ) -> Self {
        Self {
            name: name.into(),
            lane_label: lane_label.into(),
            kind: TrackKind::PianoRoll,
            notes,
            clips,
        }
    }

    fn audio(
        name: impl Into<String>,
        lane_label: impl Into<String>,
        clips: Vec<PlaylistClipData>,
    ) -> Self {
        Self {
            name: name.into(),
            lane_label: lane_label.into(),
            kind: TrackKind::Audio,
            notes: Vec::new(),
            clips,
        }
    }

    fn snapshot(&self, track_index: usize) -> PianoRollTrackSnapshot {
        PianoRollTrackSnapshot {
            track_index,
            name: self.name.clone(),
            lane_label: self.lane_label.clone(),
            kind: self.kind,
            notes: self.notes.clone(),
        }
    }
}

#[derive(Clone, Debug)]
struct PlaylistClipData {
    id: usize,
    label: String,
    start_beat: f32,
    length_beats: f32,
    muted: bool,
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
            vec![
                clip(0, "Kick Loop", 0.0, 4.0),
                clip(1, "Fill", 6.0, 2.0),
                clip(2, "Drop", 8.0, 4.0),
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
            vec![clip(3, "Bassline", 0.0, 8.0), clip(4, "Pickup", 9.0, 3.0)],
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
            vec![
                clip(5, "Pads", 2.0, 6.0),
                clip(6, "Bridge Chords", 10.0, 2.0),
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
            vec![clip(7, "Hook", 4.0, 4.0), clip(8, "Answer", 9.0, 2.0)],
        ),
        PlaylistTrack::audio(
            "Vox",
            "Audio lane",
            vec![
                clip(9, "Verse Vox", 1.0, 3.0),
                clip(10, "Chorus Vox", 8.0, 4.0),
            ],
        ),
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

fn clip(id: usize, label: &str, start_beat: f32, length_beats: f32) -> PlaylistClipData {
    PlaylistClipData {
        id,
        label: label.to_owned(),
        start_beat,
        length_beats,
        muted: false,
    }
}

fn default_clip_label(track: &PlaylistTrack) -> String {
    match track.kind {
        TrackKind::PianoRoll => format!("{} Pattern", track.name),
        TrackKind::Audio => format!("{} Audio", track.name),
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

    const fn icon(self) -> ToolIcon {
        match self {
            Self::Select => ToolIcon::Select,
            Self::Draw => ToolIcon::Draw,
            Self::Split => ToolIcon::Split,
            Self::Duplicate => ToolIcon::Duplicate,
            Self::Mute => ToolIcon::Mute,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ToolIcon {
    Select,
    Draw,
    Split,
    Duplicate,
    Mute,
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

fn draw_timeline(
    painter: &egui::Painter,
    rect: egui::Rect,
    offset_x: f32,
    viewport: PlaylistViewport,
) {
    let grid_width = (rect.width() - offset_x).max(1.0);
    let beat_width = grid_width / grid_total_beats(viewport);
    let bars = viewport.visible_bars as usize;
    for index in 0..bars {
        let x = rect.left() + offset_x + index as f32 * beat_width * 4.0;
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

fn draw_grid(
    painter: &egui::Painter,
    rect: egui::Rect,
    track_count: usize,
    viewport: PlaylistViewport,
) {
    let beat_width = beat_width(rect, viewport);
    let beats = grid_total_beats(viewport) as usize;
    for index in 0..=beats {
        let x = rect.left() + index as f32 * beat_width;
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
        let y = rect.top() - viewport.vertical_scroll + index as f32 * (TRACK_HEIGHT + TRACK_GAP);
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(1.0, theme::GRID_ROW),
        );
    }
}

fn draw_tracks(
    sidebar_painter: &egui::Painter,
    grid_painter: &egui::Painter,
    sidebar_rect: egui::Rect,
    grid_rect: egui::Rect,
    tracks: &[PlaylistTrack],
    selected_track: usize,
    selected_clip: Option<ClipSelection>,
    viewport: PlaylistViewport,
) {
    for (index, track) in tracks.iter().enumerate() {
        let top = sidebar_rect.top() - viewport.vertical_scroll
            + index as f32 * (TRACK_HEIGHT + TRACK_GAP);
        let track_rect = egui::Rect::from_min_max(
            egui::pos2(sidebar_rect.left(), top),
            egui::pos2(sidebar_rect.right(), top + TRACK_HEIGHT),
        );
        let track_lane_rect = egui::Rect::from_min_max(
            egui::pos2(grid_rect.left(), top),
            egui::pos2(grid_rect.right(), top + TRACK_HEIGHT),
        );
        let is_selected_track = selected_track == index;

        let track_fill = if is_selected_track {
            theme::TOOL_ACTIVE_WASH
        } else if index % 2 == 0 {
            theme::SURFACE_1
        } else {
            theme::SURFACE_0
        };

        sidebar_painter.rect_filled(track_rect, egui::CornerRadius::ZERO, track_fill);
        if is_selected_track {
            grid_painter.rect_filled(
                track_lane_rect,
                egui::CornerRadius::ZERO,
                egui::Color32::from_rgba_unmultiplied(
                    theme::ACCENT.r(),
                    theme::ACCENT.g(),
                    theme::ACCENT.b(),
                    12,
                ),
            );
            sidebar_painter.line_segment(
                [
                    egui::pos2(track_rect.left(), track_rect.top()),
                    egui::pos2(track_rect.left(), track_rect.bottom()),
                ],
                egui::Stroke::new(2.0, theme::ACCENT),
            );
        }

        sidebar_painter.text(
            egui::pos2(track_rect.left() + 14.0, track_rect.top() + 18.0),
            egui::Align2::LEFT_CENTER,
            track.name.as_str(),
            egui::FontId::proportional(13.0),
            theme::TEXT,
        );
        sidebar_painter.text(
            egui::pos2(track_rect.left() + 14.0, track_rect.top() + 40.0),
            egui::Align2::LEFT_CENTER,
            track.lane_label.as_str(),
            egui::FontId::proportional(11.0),
            theme::TEXT_MUTED,
        );

        let kind_tag = match track.kind {
            TrackKind::PianoRoll => "MIDI",
            TrackKind::Audio => "AUDIO",
        };
        sidebar_painter.text(
            egui::pos2(track_rect.right() - 14.0, track_rect.top() + 18.0),
            egui::Align2::RIGHT_CENTER,
            kind_tag,
            egui::FontId::proportional(10.5),
            if track.kind == TrackKind::PianoRoll {
                theme::ACCENT_HOVER
            } else {
                theme::TEXT_MUTED
            },
        );

        for clip in &track.clips {
            let clip_rect = clip_rect(grid_rect, index, clip, viewport);
            if clip_rect.width() <= 6.0 {
                continue;
            }

            let is_selected_clip = selected_clip.is_some_and(|selection| {
                selection.track_index == index && selection.clip_id == clip.id
            });

            let (fill, stroke, text_color) = clip_palette(track.kind, clip.muted, is_selected_clip);
            grid_painter.rect_filled(clip_rect, 4.0, fill);
            grid_painter.rect_stroke(
                clip_rect,
                4.0,
                egui::Stroke::new(if is_selected_clip { 1.5 } else { 1.0 }, stroke),
                egui::StrokeKind::Inside,
            );

            let handle_stroke = egui::Stroke::new(1.0, stroke);
            grid_painter.line_segment(
                [
                    egui::pos2(clip_rect.left() + 7.0, clip_rect.top() + 7.0),
                    egui::pos2(clip_rect.left() + 7.0, clip_rect.bottom() - 7.0),
                ],
                handle_stroke,
            );
            grid_painter.line_segment(
                [
                    egui::pos2(clip_rect.right() - 7.0, clip_rect.top() + 7.0),
                    egui::pos2(clip_rect.right() - 7.0, clip_rect.bottom() - 7.0),
                ],
                handle_stroke,
            );

            if clip_rect.width() > 44.0 {
                grid_painter.text(
                    egui::pos2(clip_rect.left() + 14.0, clip_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    clip.label.as_str(),
                    egui::FontId::proportional(11.5),
                    text_color,
                );
            }
            if clip.muted && clip_rect.width() > 60.0 {
                grid_painter.text(
                    egui::pos2(clip_rect.right() - 10.0, clip_rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    "M",
                    egui::FontId::proportional(11.0),
                    theme::TEXT_MUTED,
                );
            }
        }
    }
}

fn clip_palette(
    track_kind: TrackKind,
    muted: bool,
    selected: bool,
) -> (egui::Color32, egui::Color32, egui::Color32) {
    if muted {
        return (
            egui::Color32::from_rgba_unmultiplied(150, 154, 162, 42),
            egui::Color32::from_rgb(128, 133, 141),
            theme::TEXT_MUTED,
        );
    }

    match (track_kind, selected) {
        (TrackKind::PianoRoll, true) => (
            egui::Color32::from_rgba_unmultiplied(111, 184, 255, 72),
            theme::ACCENT_HOVER,
            theme::TEXT,
        ),
        (TrackKind::PianoRoll, false) => (theme::accent_soft(), theme::ACCENT, theme::TEXT),
        (TrackKind::Audio, true) => (
            egui::Color32::from_rgba_unmultiplied(196, 202, 214, 60),
            egui::Color32::from_rgb(218, 224, 235),
            theme::TEXT,
        ),
        (TrackKind::Audio, false) => (
            egui::Color32::from_rgba_unmultiplied(196, 202, 214, 28),
            theme::TEXT_MUTED,
            theme::TEXT_MUTED,
        ),
    }
}

fn icon_source(icon: ToolIcon) -> egui::ImageSource<'static> {
    match icon {
        ToolIcon::Select => egui::include_image!("../assets/piano_roll_tools/select.svg"),
        ToolIcon::Draw => egui::include_image!("../assets/piano_roll_tools/draw.svg"),
        ToolIcon::Split => egui::include_image!("../assets/piano_roll_tools/split.svg"),
        ToolIcon::Duplicate => egui::include_image!("../assets/piano_roll_tools/duplicate.svg"),
        ToolIcon::Mute => egui::include_image!("../assets/piano_roll_tools/mute.svg"),
    }
}

fn clip_hit_test(
    rect: egui::Rect,
    tracks: &[PlaylistTrack],
    pos: egui::Pos2,
    viewport: PlaylistViewport,
) -> Option<ClipHit> {
    for (track_index, track) in tracks.iter().enumerate().rev() {
        for clip in track.clips.iter().rev() {
            let clip_rect = clip_rect(rect, track_index, clip, viewport);
            if !clip_rect.contains(pos) {
                continue;
            }

            let handle_width = RESIZE_HANDLE_WIDTH.min(clip_rect.width() * 0.5);
            let edge = if pos.x <= clip_rect.left() + handle_width {
                Some(ResizeEdge::Start)
            } else if pos.x >= clip_rect.right() - handle_width {
                Some(ResizeEdge::End)
            } else {
                None
            };
            return Some(ClipHit {
                track_index,
                clip_id: clip.id,
                edge,
            });
        }
    }

    None
}

fn clip_rect(
    rect: egui::Rect,
    track_index: usize,
    clip: &PlaylistClipData,
    viewport: PlaylistViewport,
) -> egui::Rect {
    let top = rect.top() - viewport.vertical_scroll
        + track_index as f32 * (TRACK_HEIGHT + TRACK_GAP)
        + CLIP_VERTICAL_PADDING;
    let bottom = (top + TRACK_HEIGHT - CLIP_VERTICAL_PADDING * 2.0).min(rect.bottom() - 2.0);
    let beat_width = beat_width(rect, viewport);
    let left = rect.left() + clip.start_beat * beat_width;
    let right = (left + clip.length_beats * beat_width).min(rect.right() - 3.0);
    egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom))
}

fn pos_to_track(
    rect: egui::Rect,
    pos: egui::Pos2,
    track_count: usize,
    viewport: PlaylistViewport,
) -> Option<usize> {
    let local_y = pos.y - rect.top() + viewport.vertical_scroll;
    if local_y < 0.0 {
        return None;
    }

    let stride = TRACK_HEIGHT + TRACK_GAP;
    let track_index = (local_y / stride).floor() as usize;
    if track_index >= track_count {
        return None;
    }

    let track_top = track_index as f32 * stride;
    if local_y <= track_top + TRACK_HEIGHT {
        Some(track_index)
    } else {
        None
    }
}

fn pos_to_beat(rect: egui::Rect, pos: egui::Pos2, viewport: PlaylistViewport) -> f32 {
    ((pos.x - rect.left()) / beat_width(rect, viewport)).clamp(0.0, grid_total_beats(viewport))
}

fn beat_width(rect: egui::Rect, viewport: PlaylistViewport) -> f32 {
    rect.width() / grid_total_beats(viewport)
}

fn grid_total_beats(viewport: PlaylistViewport) -> f32 {
    (viewport.visible_bars * 4.0).max(MIN_CLIP_LENGTH)
}

fn playlist_content_height(track_count: usize) -> f32 {
    if track_count == 0 {
        0.0
    } else {
        track_count as f32 * TRACK_HEIGHT + (track_count - 1) as f32 * TRACK_GAP
    }
}

fn zoom_visible_bars(current: f32, scroll_y: f32) -> f32 {
    (current * (-scroll_y * ZOOM_SENSITIVITY).exp()).clamp(MIN_VISIBLE_BARS, MAX_VISIBLE_BARS)
}

fn quantize_beat(beat: f32, free_drag: bool) -> f32 {
    if free_drag {
        beat.max(0.0)
    } else {
        (beat / SNAP_BEATS).round() * SNAP_BEATS
    }
}

fn clamp_start_beat(start: f32, length: f32, grid_beats: f32) -> f32 {
    start.clamp(0.0, (grid_beats - length.max(MIN_CLIP_LENGTH)).max(0.0))
}

fn clamp_clip_length(length: f32, start: f32, grid_beats: f32) -> f32 {
    length
        .max(MIN_CLIP_LENGTH)
        .min((grid_beats - start).max(MIN_CLIP_LENGTH))
}

fn sort_clips(clips: &mut [PlaylistClipData]) {
    clips.sort_by(|left, right| {
        left.start_beat
            .total_cmp(&right.start_beat)
            .then(left.length_beats.total_cmp(&right.length_beats))
            .then(left.id.cmp(&right.id))
    });
}
