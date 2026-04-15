use std::sync::Arc;
use std::time::Instant;

use daw_audio_engine::{
    AudioEngine, MidiClip, MidiNote, MidiTrack, SimpleSynth, Timeline, TimelineHandle,
    TimelineProcessor, TransportMonitor, TransportState, VirtualInstrument, DEFAULT_SAMPLE_RATE,
};
use eframe::egui;
use parking_lot::Mutex;

use crate::{
    explorer::ExplorerView,
    piano_roll::{lane_to_midi_note, PianoRollView},
    playlist::{ArrangementTrackSnapshot, PlaylistView, TrackKind},
    theme,
};

const APP_TOOLBAR_HEIGHT: f32 = 46.0;
const EXPLORER_WIDTH: f32 = 272.0;
const DEFAULT_BPM: f32 = 120.0;
const DEFAULT_LOOP_BARS: f32 = 4.0;

pub struct DawUiApp {
    audio: AudioSession,
    explorer: ExplorerView,
    piano_roll: PianoRollView,
    piano_roll_open: bool,
    playlist: PlaylistView,
}

struct AudioSession {
    backend: AudioBackend,
    synced_revision: u64,
}

enum AudioBackend {
    Ready(ReadyAudioSession),
    Unavailable(String),
}

struct ReadyAudioSession {
    engine: AudioEngine,
    timeline: Arc<Mutex<Timeline>>,
    timeline_handle: TimelineHandle,
    transport_monitor: TransportMonitor,
    bpm: f32,
    loop_length_beats: f32,
    ui_transport: UiTransportState,
}

struct UiTransportState {
    observed_position_seconds: f64,
    observed_at: Instant,
    observed_state: TransportState,
    observed_bpm: f32,
}

impl DawUiApp {
    pub fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&creation_context.egui_ctx);
        egui_extras::install_image_loaders(&creation_context.egui_ctx);
        Self {
            audio: AudioSession::new(),
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
                if toolbar_button(ui, "Play", self.audio.is_playing()).clicked() {
                    self.audio.toggle_playback();
                }
                if toolbar_button(ui, "Stop", false).clicked() {
                    self.audio.stop();
                }

                toolbar_chip(
                    ui,
                    &format!("Beat {:.2}", self.audio.playhead_beats() + 1.0),
                );
                toolbar_chip(
                    ui,
                    &format!("Loop {:.0} Bars", self.audio.loop_length_beats() / 4.0),
                );
                toolbar_chip(ui, &format!("{:.0} BPM", self.audio.bpm()));
                toolbar_chip(ui, self.audio.status_label());
            });
        });
    }
}

impl eframe::App for DawUiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.audio.is_playing() {
            ctx.request_repaint();
        }

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

                let playhead_beats = self.audio.playhead_beats();
                let mut active_track = self.playlist.active_track_snapshot();
                let piano_roll_tracks = self.playlist.piano_roll_tracks();
                let piano_roll_output = self.piano_roll.show_window(
                    ctx,
                    &mut self.piano_roll_open,
                    &mut active_track,
                    &piano_roll_tracks,
                    playhead_beats,
                );
                if let Some(seek_to_beat) = piano_roll_output.seek_to_beat {
                    self.audio.seek_beats(seek_to_beat);
                }
                self.playlist.set_active_track_notes(active_track.notes);

                ui.scope_builder(egui::UiBuilder::new().max_rect(workspace_rect), |ui| {
                    if let Some(seek_to_beat) =
                        self.playlist
                            .show(ui, !piano_roll_output.blocks_pointer, playhead_beats)
                    {
                        self.audio.seek_beats(seek_to_beat);
                    }
                });

                self.audio.sync_arrangement(
                    DEFAULT_BPM,
                    self.playlist.revision(),
                    &self.playlist.arrangement_tracks(),
                );
            });
    }
}

impl AudioSession {
    fn new() -> Self {
        let mut engine = match AudioEngine::new() {
            Ok(engine) => engine,
            Err(error) => {
                return Self {
                    backend: AudioBackend::Unavailable(format!("Audio init failed: {error}")),
                    synced_revision: u64::MAX,
                };
            }
        };

        let sample_rate = engine
            .current_device()
            .map(|device| device.sample_rate)
            .unwrap_or(DEFAULT_SAMPLE_RATE);
        let mut timeline = Timeline::with_bpm("DAW", sample_rate, DEFAULT_BPM as f64);
        timeline.sequencer.transport_mut().set_loop(true);
        timeline
            .sequencer
            .transport_mut()
            .set_loop_range(0.0, beats_to_seconds(DEFAULT_LOOP_BARS * 4.0, DEFAULT_BPM));

        let (processor, timeline_handle) =
            TimelineProcessor::new(timeline, Box::new(SimpleSynth::new(32)));
        let timeline = processor.timeline_arc();
        let transport_monitor = {
            let timeline = timeline.lock();
            timeline.sequencer.transport().get_monitor()
        };

        engine.set_processor(Box::new(processor));
        if let Err(error) = engine.start() {
            return Self {
                backend: AudioBackend::Unavailable(format!("Audio offline: {error}")),
                synced_revision: u64::MAX,
            };
        }

        Self {
            backend: AudioBackend::Ready(ReadyAudioSession {
                engine,
                timeline,
                timeline_handle,
                transport_monitor,
                bpm: DEFAULT_BPM,
                loop_length_beats: DEFAULT_LOOP_BARS * 4.0,
                ui_transport: UiTransportState::new(DEFAULT_BPM),
            }),
            synced_revision: u64::MAX,
        }
    }

    fn sync_arrangement(
        &mut self,
        bpm: f32,
        revision: u64,
        arrangement_tracks: &[ArrangementTrackSnapshot],
    ) {
        let AudioBackend::Ready(backend) = &mut self.backend else {
            return;
        };
        if self.synced_revision == revision && (backend.bpm - bpm).abs() <= f32::EPSILON {
            return;
        }

        let loop_length_beats = arrangement_tracks
            .iter()
            .flat_map(|track| track.clips.iter())
            .map(|clip| clip.start_beat + clip.length_beats)
            .fold(DEFAULT_LOOP_BARS * 4.0, f32::max)
            .max(1.0);

        let mut timeline = backend.timeline.lock();
        timeline.sequencer.transport_mut().set_bpm(bpm as f64);
        timeline.sequencer.transport_mut().set_loop(true);
        timeline
            .sequencer
            .transport_mut()
            .set_loop_range(0.0, beats_to_seconds(loop_length_beats, bpm));
        timeline.sequencer.clear_tracks();

        for arrangement_track in arrangement_tracks
            .iter()
            .filter(|track| track.kind == TrackKind::PianoRoll)
        {
            let instrument: Arc<dyn VirtualInstrument> = Arc::new(SimpleSynth::new(32));
            let mut track = MidiTrack::new(&arrangement_track.name, instrument);
            track.muted = arrangement_track.muted;

            for clip_snapshot in arrangement_track.clips.iter().filter(|clip| !clip.muted) {
                let mut clip = MidiClip::new(
                    &clip_snapshot.label,
                    beats_to_seconds(clip_snapshot.start_beat, bpm),
                    beats_to_seconds(clip_snapshot.length_beats, bpm),
                );
                for note in &arrangement_track.notes {
                    clip.add_note(MidiNote::new(
                        lane_to_midi_note(note.lane),
                        note_velocity_to_midi(note.velocity),
                        beats_to_seconds(note.start_beat, bpm),
                        beats_to_seconds(note.length_beats, bpm),
                    ));
                }
                track.add_clip(clip);
            }

            timeline.sequencer.add_track(track);
        }

        backend.bpm = bpm;
        backend.loop_length_beats = loop_length_beats;
        self.synced_revision = revision;
    }

    fn toggle_playback(&mut self) {
        if let AudioBackend::Ready(backend) = &mut self.backend {
            match backend.transport_monitor.state() {
                TransportState::Playing => {
                    backend.timeline_handle.pause();
                    backend.ui_transport.observe(
                        backend.transport_monitor.position(),
                        TransportState::Paused,
                        backend.transport_monitor.bpm() as f32,
                    );
                }
                _ => {
                    backend.timeline_handle.play();
                    backend.ui_transport.observe(
                        backend.transport_monitor.position(),
                        TransportState::Playing,
                        backend.transport_monitor.bpm() as f32,
                    );
                }
            }
        }
    }

    fn stop(&mut self) {
        if let AudioBackend::Ready(backend) = &mut self.backend {
            backend.timeline_handle.stop();
            backend.ui_transport.observe(
                0.0,
                TransportState::Stopped,
                backend.transport_monitor.bpm() as f32,
            );
        }
    }

    fn seek_beats(&mut self, beats: f32) {
        if let AudioBackend::Ready(backend) = &mut self.backend {
            let bpm = backend.transport_monitor.bpm() as f32;
            let seconds = beats_to_seconds(beats, bpm);
            backend.timeline_handle.seek(seconds);
            backend
                .ui_transport
                .observe(seconds, backend.transport_monitor.state(), bpm);
        }
    }

    fn playhead_beats(&mut self) -> f32 {
        match &mut self.backend {
            AudioBackend::Ready(backend) => {
                backend.ui_transport.observe(
                    backend.transport_monitor.position(),
                    backend.transport_monitor.state(),
                    backend.transport_monitor.bpm() as f32,
                );
                let loop_length_seconds =
                    beats_to_seconds(backend.loop_length_beats, backend.ui_transport.observed_bpm);
                seconds_to_beats(
                    backend
                        .ui_transport
                        .estimated_position_seconds(loop_length_seconds),
                    backend.ui_transport.observed_bpm,
                )
            }
            AudioBackend::Unavailable(_) => 0.0,
        }
    }

    fn bpm(&mut self) -> f32 {
        match &mut self.backend {
            AudioBackend::Ready(backend) => {
                backend.ui_transport.observe(
                    backend.transport_monitor.position(),
                    backend.transport_monitor.state(),
                    backend.transport_monitor.bpm() as f32,
                );
                backend.ui_transport.observed_bpm
            }
            AudioBackend::Unavailable(_) => DEFAULT_BPM,
        }
    }

    fn loop_length_beats(&self) -> f32 {
        match &self.backend {
            AudioBackend::Ready(backend) => backend.loop_length_beats,
            AudioBackend::Unavailable(_) => DEFAULT_LOOP_BARS * 4.0,
        }
    }

    fn is_playing(&self) -> bool {
        matches!(
            &self.backend,
            AudioBackend::Ready(backend)
                if backend.transport_monitor.state() == TransportState::Playing
        )
    }

    fn status_label(&self) -> &str {
        match &self.backend {
            AudioBackend::Ready(backend) => {
                if backend.engine.is_running() {
                    "Audio Live"
                } else {
                    "Audio Idle"
                }
            }
            AudioBackend::Unavailable(message) => message.as_str(),
        }
    }
}

fn beats_to_seconds(beats: f32, bpm: f32) -> f64 {
    beats as f64 * 60.0 / bpm.max(1.0) as f64
}

fn seconds_to_beats(seconds: f64, bpm: f32) -> f32 {
    (seconds * bpm.max(1.0) as f64 / 60.0) as f32
}

fn note_velocity_to_midi(velocity: f32) -> u8 {
    (velocity.clamp(0.0, 1.0) * 127.0).round().clamp(1.0, 127.0) as u8
}

impl UiTransportState {
    fn new(bpm: f32) -> Self {
        Self {
            observed_position_seconds: 0.0,
            observed_at: Instant::now(),
            observed_state: TransportState::Stopped,
            observed_bpm: bpm,
        }
    }

    fn observe(&mut self, position_seconds: f64, state: TransportState, bpm: f32) {
        let should_reset_clock = state != self.observed_state
            || bpm != self.observed_bpm
            || position_seconds < self.observed_position_seconds
            || (position_seconds - self.observed_position_seconds).abs() > 0.075;

        self.observed_position_seconds = position_seconds.max(0.0);
        self.observed_state = state;
        self.observed_bpm = bpm.max(1.0);
        if should_reset_clock {
            self.observed_at = Instant::now();
        }
    }

    fn estimated_position_seconds(&self, loop_length_seconds: f64) -> f64 {
        let mut position = self.observed_position_seconds;
        if self.observed_state == TransportState::Playing {
            position += self.observed_at.elapsed().as_secs_f64();
        }

        if loop_length_seconds > 0.0 {
            position %= loop_length_seconds;
        }
        position.max(0.0)
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
