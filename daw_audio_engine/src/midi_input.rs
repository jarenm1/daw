use crate::midi_event::MidiEvent;
use crossbeam::channel::{bounded, Receiver, Sender};
use log::{error, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// MIDI input handler for hardware keyboards/controllers
pub struct MidiInput {
    sender: Sender<MidiEvent>,
    receiver: Receiver<MidiEvent>,
    running: Arc<AtomicBool>,
    _connection: Option<midir::MidiInputConnection<()>>,
}

impl MidiInput {
    pub fn new() -> Self {
        let (sender, receiver) = bounded(1024); // Buffer up to 1024 MIDI events

        Self {
            sender,
            receiver,
            running: Arc::new(AtomicBool::new(false)),
            _connection: None,
        }
    }

    /// List available MIDI input ports
    pub fn list_ports() -> Vec<(usize, String)> {
        let midi_in = match midir::MidiInput::new("DAW Audio Engine") {
            Ok(m) => m,
            Err(_) => return Vec::new(),
        };

        midi_in
            .ports()
            .iter()
            .enumerate()
            .filter_map(|(i, port)| midi_in.port_name(port).ok().map(|name| (i, name)))
            .collect()
    }

    /// Connect to a MIDI input port by index
    pub fn connect(&mut self, port_index: usize) -> Result<(), String> {
        let midi_in = midir::MidiInput::new("DAW Audio Engine")
            .map_err(|e| format!("Failed to create MIDI input: {}", e))?;

        let ports = midi_in.ports();
        if port_index >= ports.len() {
            return Err(format!("Invalid port index: {}", port_index));
        }

        let port = &ports[port_index];
        let port_name = midi_in.port_name(port).map_err(|e| e.to_string())?;

        let sender = self.sender.clone();
        let running = self.running.clone();
        running.store(true, Ordering::SeqCst);

        info!("Connecting to MIDI input: {}", port_name);

        let connection = midi_in
            .connect(
                port,
                "daw-input",
                move |_timestamp, message, _| {
                    if let Some(event) = Self::parse_midi_message(message) {
                        let _ = sender.try_send(event);
                    }
                },
                (),
            )
            .map_err(|e| format!("Failed to connect: {}", e))?;

        self._connection = Some(connection);
        info!("MIDI input connected successfully");

        Ok(())
    }

    /// Try to connect to first available port
    pub fn connect_first_available(&mut self) -> Result<(), String> {
        let ports = Self::list_ports();
        if ports.is_empty() {
            return Err("No MIDI input ports available".to_string());
        }

        info!("Available MIDI ports: {:?}", ports);
        self.connect(ports[0].0)
    }

    /// Disconnect from MIDI input
    pub fn disconnect(&mut self) {
        self._connection = None;
        self.running.store(false, Ordering::SeqCst);
        info!("MIDI input disconnected");
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self._connection.is_some()
    }

    /// Receive pending MIDI events (call from audio thread)
    pub fn recv_events(&self) -> Vec<MidiEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.receiver.try_recv() {
            events.push(event);
        }
        events
    }

    /// Get receiver for manual event polling
    pub fn receiver(&self) -> &Receiver<MidiEvent> {
        &self.receiver
    }

    /// Parse raw MIDI bytes to MidiEvent
    fn parse_midi_message(bytes: &[u8]) -> Option<MidiEvent> {
        if bytes.is_empty() {
            return None;
        }

        let status = bytes[0];
        let channel = status & 0x0F;
        let message_type = status & 0xF0;

        use crate::midi_event::MidiEventType;

        let event_type = match message_type {
            0x80 => {
                // Note Off
                if bytes.len() >= 3 {
                    MidiEventType::NoteOff { pitch: bytes[1] }
                } else {
                    return None;
                }
            }
            0x90 => {
                // Note On (velocity 0 = note off)
                if bytes.len() >= 3 {
                    MidiEventType::NoteOn {
                        pitch: bytes[1],
                        velocity: bytes[2],
                    }
                } else {
                    return None;
                }
            }
            0xB0 => {
                // Control Change
                if bytes.len() >= 3 {
                    MidiEventType::ControlChange {
                        controller: bytes[1],
                        value: bytes[2],
                    }
                } else {
                    return None;
                }
            }
            0xE0 => {
                // Pitch Bend (14-bit value)
                if bytes.len() >= 3 {
                    let value = ((bytes[2] as i16) << 7 | bytes[1] as i16) - 8192;
                    MidiEventType::PitchBend { value }
                } else {
                    return None;
                }
            }
            0xA0 => {
                // Polyphonic Aftertouch
                if bytes.len() >= 3 {
                    MidiEventType::Aftertouch {
                        pitch: bytes[1],
                        pressure: bytes[2],
                    }
                } else {
                    return None;
                }
            }
            0xD0 => {
                // Channel Pressure
                if bytes.len() >= 2 {
                    MidiEventType::ChannelPressure { pressure: bytes[1] }
                } else {
                    return None;
                }
            }
            0xC0 => {
                // Program Change
                if bytes.len() >= 2 {
                    MidiEventType::ProgramChange { program: bytes[1] }
                } else {
                    return None;
                }
            }
            _ => {
                // System messages, etc. - ignore for now
                return None;
            }
        };

        Some(MidiEvent {
            event_type,
            channel,
            timestamp: 0, // Will be set by audio engine for sample-accurate timing
        })
    }
}

impl Drop for MidiInput {
    fn drop(&mut self) {
        self.disconnect();
    }
}
