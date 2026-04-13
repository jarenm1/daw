use crate::instrument::VirtualInstrument;
use crate::midi_event::MidiEvent;
use crate::processor::{AudioProcessor, ProcessorConfig};
use crate::timeline::Timeline;
use crossbeam::channel::{bounded, Receiver, Sender};
use parking_lot::Mutex;
use std::sync::Arc;

/// Audio processor that combines timeline sequencing with instrument synthesis
/// This runs in the audio callback and ensures perfect sync between transport and audio
pub struct TimelineProcessor {
    timeline: Arc<Mutex<Timeline>>,
    instrument: Box<dyn VirtualInstrument>,
    config: Option<ProcessorConfig>,

    // Channel for UI MIDI events (note preview, etc.)
    midi_receiver: Receiver<MidiEvent>,

    // Channel for transport commands
    cmd_receiver: Receiver<TransportCommand>,

    // Event counter for debugging
    event_count: u64,
}

/// Transport control commands
#[derive(Debug, Clone)]
pub enum TransportCommand {
    Play,
    Pause,
    Stop,
    Seek(f64),
}

/// Handle for sending UI events to the timeline processor
#[derive(Clone)]
pub struct TimelineHandle {
    midi_sender: Sender<MidiEvent>,
    cmd_sender: Sender<TransportCommand>,
}

impl TimelineHandle {
    /// Send a MIDI event for immediate playback (preview)
    pub fn send_event(&self, event: MidiEvent) {
        let _ = self.midi_sender.try_send(event);
    }

    /// Preview a note (convenience method)
    pub fn preview_note(&self, pitch: u8, velocity: u8) {
        let event = MidiEvent::note_on(pitch, velocity, 0);
        self.send_event(event);
    }

    /// Stop previewing a note
    pub fn stop_note(&self, pitch: u8) {
        let event = MidiEvent::note_off(pitch, 0);
        self.send_event(event);
    }

    /// Start playback
    pub fn play(&self) {
        let _ = self.cmd_sender.try_send(TransportCommand::Play);
    }

    /// Pause playback
    pub fn pause(&self) {
        let _ = self.cmd_sender.try_send(TransportCommand::Pause);
    }

    /// Stop playback
    pub fn stop(&self) {
        let _ = self.cmd_sender.try_send(TransportCommand::Stop);
    }

    /// Seek to position
    pub fn seek(&self, position: f64) {
        let _ = self.cmd_sender.try_send(TransportCommand::Seek(position));
    }
}

impl TimelineProcessor {
    /// Create a new timeline processor
    pub fn new(
        timeline: Timeline,
        instrument: Box<dyn VirtualInstrument>,
    ) -> (Self, TimelineHandle) {
        let (midi_tx, midi_rx) = bounded(256);
        let (cmd_tx, cmd_rx) = bounded(64);

        let handle = TimelineHandle {
            midi_sender: midi_tx,
            cmd_sender: cmd_tx,
        };

        let processor = Self {
            timeline: Arc::new(Mutex::new(timeline)),
            instrument,
            config: None,
            midi_receiver: midi_rx,
            cmd_receiver: cmd_rx,
            event_count: 0,
        };

        (processor, handle)
    }

    /// Get the timeline Arc for GUI access
    pub fn timeline_arc(&self) -> Arc<Mutex<Timeline>> {
        self.timeline.clone()
    }

    /// Play the timeline
    pub fn play(&self) {
        self.timeline.lock().play();
    }

    /// Pause the timeline
    pub fn pause(&self) {
        self.timeline.lock().pause();
    }

    /// Stop the timeline
    pub fn stop(&self) {
        self.timeline.lock().stop();
    }

    /// Seek to position
    pub fn seek(&self, position: f64) {
        self.timeline.lock().seek(position);
    }

    /// Get current transport position
    pub fn position(&self) -> f64 {
        self.timeline.lock().sequencer.transport().position()
    }

    /// Check if playing
    pub fn is_playing(&self) -> bool {
        self.timeline.lock().sequencer.transport().is_playing()
    }
}

impl AudioProcessor for TimelineProcessor {
    fn process(&mut self, _input: &[f32], output: &mut [f32]) {
        let buffer_size = output.len();
        let channels = self.config.as_ref().map(|c| c.output_channels).unwrap_or(2);
        let samples_per_channel = buffer_size / channels;
        let sample_rate = self
            .config
            .as_ref()
            .map(|c| c.sample_rate as u32)
            .unwrap_or(48000);

        // Process transport commands from UI
        let mut was_stopped = false;
        while let Ok(cmd) = self.cmd_receiver.try_recv() {
            match cmd {
                TransportCommand::Play => self.timeline.lock().play(),
                TransportCommand::Pause => {
                    self.timeline.lock().pause();
                    was_stopped = true;
                }
                TransportCommand::Stop => {
                    self.timeline.lock().stop();
                    was_stopped = true;
                }
                TransportCommand::Seek(pos) => {
                    self.timeline.lock().seek(pos);
                    was_stopped = true;
                }
            }
        }

        // Kill all voices when stopping to prevent stuck notes
        if was_stopped {
            self.instrument.reset();
            eprintln!("[AUDIO] All voices reset due to stop/pause/seek");
        }

        // Process timeline to generate MIDI events
        // This must happen in the audio callback for perfect sync
        let timeline_events = {
            let mut timeline = self.timeline.lock();
            timeline.process(samples_per_channel)
        };

        // Send timeline events to instrument
        if !timeline_events.is_empty() {
            for event in timeline_events {
                eprintln!(
                    "[TIMELINE] Event at {:.3}s: {:?}",
                    self.position(),
                    event.event_type
                );
                self.instrument.handle_event(&event);
                self.event_count += 1;
            }
        }

        // Process UI MIDI events (note preview, etc.)
        let mut ui_events = 0;
        while let Ok(event) = self.midi_receiver.try_recv() {
            self.instrument.handle_event(&event);
            self.event_count += 1;
            ui_events += 1;
        }

        if ui_events > 0 {
            eprintln!("[UI] Processed {} preview events", ui_events);
        }

        // Render audio
        self.instrument.render(output, channels, sample_rate);
    }

    fn configure(&mut self, config: &ProcessorConfig) {
        self.config = Some(ProcessorConfig {
            sample_rate: config.sample_rate,
            buffer_size: config.buffer_size,
            input_channels: config.input_channels,
            output_channels: config.output_channels,
        });
    }

    fn name(&self) -> &str {
        "TimelineProcessor"
    }
}
