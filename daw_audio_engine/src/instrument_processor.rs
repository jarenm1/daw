use crate::instrument::VirtualInstrument;
use crate::midi_event::MidiEvent;
use crate::processor::{AudioProcessor, ProcessorConfig};
use crossbeam::channel::{bounded, Receiver, Sender};
use parking_lot::Mutex;
use std::sync::Arc;

/// Audio processor that wraps a virtual instrument with MIDI input
/// Handles both live MIDI input and preview events from UI
pub struct InstrumentProcessor {
    instrument: Arc<Mutex<Box<dyn VirtualInstrument>>>,
    config: Option<ProcessorConfig>,

    // Channels for MIDI events
    live_receiver: Receiver<MidiEvent>, // From hardware MIDI input
    preview_receiver: Receiver<MidiEvent>, // From UI (piano roll clicks)

    // For monitoring/feedback
    event_count: u64,
}

/// Handle for sending preview events to the instrument from UI
#[derive(Clone)]
pub struct PreviewHandle {
    sender: Sender<MidiEvent>,
}

impl PreviewHandle {
    /// Send a MIDI event for immediate playback (preview)
    pub fn send_event(&self, event: MidiEvent) {
        let _ = self.sender.try_send(event);
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
}

impl InstrumentProcessor {
    /// Create a new instrument processor
    pub fn new(instrument: Box<dyn VirtualInstrument>) -> (Self, PreviewHandle) {
        let (live_tx, live_rx) = bounded(256);
        let (preview_tx, preview_rx) = bounded(64); // Smaller buffer for UI events

        let _ = live_tx; // Keep sender alive
        let preview_handle = PreviewHandle { sender: preview_tx };

        let processor = Self {
            instrument: Arc::new(Mutex::new(instrument)),
            config: None,
            live_receiver: live_rx,
            preview_receiver: preview_rx,
            event_count: 0,
        };

        (processor, preview_handle)
    }

    /// Get a clone of the instrument Arc
    pub fn instrument_arc(&self) -> Arc<Mutex<Box<dyn VirtualInstrument>>> {
        self.instrument.clone()
    }

    /// Get a sender for connecting to MIDI input
    pub fn create_live_sender(&self) -> Sender<MidiEvent> {
        // This would be used to connect the MIDI input module
        // For now we create a new channel pair
        let (tx, rx) = bounded(256);
        // In practice, we'd store rx and use it
        tx
    }

    /// Connect to a MIDI input receiver
    pub fn connect_midi_input(&mut self, receiver: Receiver<MidiEvent>) {
        // Replace the live receiver with the provided one
        self.live_receiver = receiver;
    }

    /// Get the wrapped instrument name
    pub fn instrument_name(&self) -> String {
        self.instrument.lock().name().to_string()
    }

    /// Reset the instrument
    pub fn reset(&mut self) {
        self.instrument.lock().reset();
        self.event_count = 0;
    }
}

impl AudioProcessor for InstrumentProcessor {
    fn process(&mut self, _input: &[f32], output: &mut [f32]) {
        // Debug: log when audio callback runs
        static mut CALL_COUNT: u32 = 0;
        unsafe {
            CALL_COUNT += 1;
            if CALL_COUNT % 100 == 0 {
                eprintln!(
                    "[AUDIO CB] Process called {} times, output len={}",
                    CALL_COUNT,
                    output.len()
                );
            }
        }

        let channels = self.config.as_ref().map(|c| c.output_channels).unwrap_or(2);
        let sample_rate = self
            .config
            .as_ref()
            .map(|c| c.sample_rate as u32)
            .unwrap_or(48000);

        // Process all pending MIDI events from both sources
        // 1. Live MIDI input (from hardware keyboard)
        while let Ok(event) = self.live_receiver.try_recv() {
            self.instrument.lock().handle_event(&event);
            self.event_count += 1;
        }

        // 2. Preview events (from UI clicks in piano roll and timeline playback)
        let mut preview_events = 0;
        while let Ok(event) = self.preview_receiver.try_recv() {
            eprintln!("[INST] Received preview event: {:?}", event.event_type);
            self.instrument.lock().handle_event(&event);
            self.event_count += 1;
            preview_events += 1;
        }

        // Render audio
        self.instrument.lock().render(output, channels, sample_rate);
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
        "InstrumentProcessor"
    }
}
