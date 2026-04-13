/// MIDI event types for real-time processing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MidiEventType {
    NoteOn { pitch: u8, velocity: u8 },
    NoteOff { pitch: u8 },
    ControlChange { controller: u8, value: u8 },
    PitchBend { value: i16 }, // -8192 to 8191
    Aftertouch { pitch: u8, pressure: u8 },
    ChannelPressure { pressure: u8 },
    ProgramChange { program: u8 },
    ModWheel { value: u8 },
}

/// A MIDI event with timing information
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MidiEvent {
    pub event_type: MidiEventType,
    pub channel: u8,    // 0-15 (MIDI channel 1-16)
    pub timestamp: u64, // Sample frame timestamp for sample-accurate timing
}

impl MidiEvent {
    pub fn note_on(pitch: u8, velocity: u8, channel: u8) -> Self {
        Self {
            event_type: MidiEventType::NoteOn { pitch, velocity },
            channel: channel.min(15),
            timestamp: 0,
        }
    }

    pub fn note_off(pitch: u8, channel: u8) -> Self {
        Self {
            event_type: MidiEventType::NoteOff { pitch },
            channel: channel.min(15),
            timestamp: 0,
        }
    }

    pub fn control_change(controller: u8, value: u8, channel: u8) -> Self {
        Self {
            event_type: MidiEventType::ControlChange { controller, value },
            channel: channel.min(15),
            timestamp: 0,
        }
    }

    pub fn pitch_bend(value: i16, channel: u8) -> Self {
        Self {
            event_type: MidiEventType::PitchBend { value },
            channel: channel.min(15),
            timestamp: 0,
        }
    }

    /// Create a preview event (immediate, no timestamp)
    pub fn preview_note_on(pitch: u8, velocity: u8) -> Self {
        Self::note_on(pitch, velocity, 0)
    }

    pub fn preview_note_off(pitch: u8) -> Self {
        Self::note_off(pitch, 0)
    }

    /// Get the pitch if this is a note event
    pub fn pitch(&self) -> Option<u8> {
        match self.event_type {
            MidiEventType::NoteOn { pitch, .. } => Some(pitch),
            MidiEventType::NoteOff { pitch } => Some(pitch),
            MidiEventType::Aftertouch { pitch, .. } => Some(pitch),
            _ => None,
        }
    }

    /// Get velocity if this is a note-on event
    pub fn velocity(&self) -> Option<u8> {
        match self.event_type {
            MidiEventType::NoteOn { velocity, .. } => Some(velocity),
            _ => None,
        }
    }

    /// Check if this is a note-on with velocity > 0
    pub fn is_note_on(&self) -> bool {
        matches!(self.event_type, MidiEventType::NoteOn { velocity, .. } if velocity > 0)
    }

    /// Check if this is a note-off or note-on with velocity 0
    pub fn is_note_off(&self) -> bool {
        matches!(self.event_type, MidiEventType::NoteOff { .. })
            || matches!(self.event_type, MidiEventType::NoteOn { velocity: 0, .. })
    }
}

/// Represents a MIDI note in a clip/sequence
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MidiNote {
    pub pitch: u8,       // 0-127 (C-1 to G9)
    pub velocity: u8,    // 0-127
    pub start_time: f64, // In seconds from clip start
    pub duration: f64,   // In seconds
    pub channel: u8,     // 0-15
}

impl MidiNote {
    pub fn new(pitch: u8, velocity: u8, start: f64, duration: f64) -> Self {
        Self {
            pitch: pitch.min(127),
            velocity: velocity.min(127),
            start_time: start,
            duration: duration.max(0.0),
            channel: 0,
        }
    }

    /// Convert to note-on event at given sample rate
    pub fn to_note_on_event(&self, sample_rate: u32) -> MidiEvent {
        let timestamp = (self.start_time * sample_rate as f64) as u64;
        MidiEvent {
            event_type: MidiEventType::NoteOn {
                pitch: self.pitch,
                velocity: self.velocity,
            },
            channel: self.channel,
            timestamp,
        }
    }

    /// Convert to note-off event at given sample rate
    pub fn to_note_off_event(&self, sample_rate: u32) -> MidiEvent {
        let end_time = self.start_time + self.duration;
        let timestamp = (end_time * sample_rate as f64) as u64;
        MidiEvent {
            event_type: MidiEventType::NoteOff { pitch: self.pitch },
            channel: self.channel,
            timestamp,
        }
    }

    /// Check if note is active at given time
    pub fn is_active_at(&self, time: f64) -> bool {
        time >= self.start_time && time < self.start_time + self.duration
    }
}

/// MIDI pitch to frequency conversion
pub fn pitch_to_freq(pitch: u8) -> f32 {
    // A4 = 69 = 440Hz
    const A4_PITCH: i16 = 69;
    const A4_FREQ: f32 = 440.0;

    let semitones = pitch as i16 - A4_PITCH;
    A4_FREQ * 2.0f32.powf(semitones as f32 / 12.0)
}

/// Frequency to pitch (rounded to nearest)
pub fn freq_to_pitch(freq: f32) -> u8 {
    const A4_FREQ: f32 = 440.0;
    let semitones = 12.0 * (freq / A4_FREQ).log2();
    (69.0 + semitones).round().clamp(0.0, 127.0) as u8
}
