//! Note data model
//!
//! Represents a single musical note with pitch, velocity, and timing.

use crate::BeatTime;
use serde::{Deserialize, Serialize};

/// MIDI note number (0-127)
pub type Pitch = u8;

/// MIDI velocity (0-127)
pub type Velocity = u8;

/// A musical note in a pattern
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Note {
    /// MIDI note number (0-127, where 60 is middle C)
    pub pitch: Pitch,
    /// Velocity (0-127)
    pub velocity: Velocity,
    /// Position in beats from pattern start
    pub start_beat: BeatTime,
    /// Duration in beats
    pub duration_beats: BeatTime,
}

impl Note {
    /// Create a new note
    pub fn new(
        pitch: Pitch,
        velocity: Velocity,
        start_beat: BeatTime,
        duration_beats: BeatTime,
    ) -> Self {
        Self {
            pitch,
            velocity,
            start_beat,
            duration_beats,
        }
    }

    /// Get the end position of this note
    pub fn end_beat(&self) -> BeatTime {
        self.start_beat + self.duration_beats
    }

    /// Check if this note overlaps with a given time range
    pub fn overlaps(&self, start: BeatTime, end: BeatTime) -> bool {
        self.start_beat < end && self.end_beat() > start
    }

    /// Check if a beat position is within this note
    pub fn contains(&self, beat: BeatTime) -> bool {
        beat >= self.start_beat && beat < self.end_beat()
    }

    /// Convert pitch to frequency in Hz
    pub fn frequency(&self) -> f64 {
        pitch_to_freq(self.pitch)
    }

    /// Get the note name (e.g., "C4", "F#5")
    pub fn name(&self) -> String {
        pitch_to_name(self.pitch)
    }
}

/// Convert MIDI pitch to frequency in Hz
pub fn pitch_to_freq(pitch: Pitch) -> f64 {
    440.0 * 2.0_f64.powf((pitch as f64 - 69.0) / 12.0)
}

/// Convert frequency in Hz to MIDI pitch
pub fn freq_to_pitch(freq: f64) -> Pitch {
    (12.0 * (freq / 440.0).log2() + 69.0).round() as Pitch
}

/// Convert MIDI pitch to note name
pub fn pitch_to_name(pitch: Pitch) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = (pitch / 12) as i8 - 1;
    let note_idx = (pitch % 12) as usize;
    format!("{}{}", NAMES[note_idx], octave)
}

/// Convert note name to MIDI pitch
pub fn name_to_pitch(name: &str) -> Option<Pitch> {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];

    if name.len() < 2 {
        return None;
    }

    let note_part = &name[..name.len() - 1];
    let octave_part = &name[name.len() - 1..];

    let octave = octave_part.parse::<i8>().ok()?;
    let note_idx = NAMES
        .iter()
        .position(|&n| n.eq_ignore_ascii_case(note_part))?;

    Some(((octave + 1) * 12 + note_idx as i8) as Pitch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pitch_to_freq() {
        assert!((pitch_to_freq(69) - 440.0).abs() < 0.01); // A4 = 440Hz
        assert!((pitch_to_freq(60) - 261.63).abs() < 0.01); // C4 ≈ 261.63Hz
    }

    #[test]
    fn test_freq_to_pitch() {
        assert_eq!(freq_to_pitch(440.0), 69);
        assert_eq!(freq_to_pitch(261.63), 60);
    }

    #[test]
    fn test_pitch_to_name() {
        assert_eq!(pitch_to_name(60), "C4");
        assert_eq!(pitch_to_name(61), "C#4");
        assert_eq!(pitch_to_name(69), "A4");
    }

    #[test]
    fn test_name_to_pitch() {
        assert_eq!(name_to_pitch("C4"), Some(60));
        assert_eq!(name_to_pitch("A4"), Some(69));
        assert_eq!(name_to_pitch("C#4"), Some(61));
        assert_eq!(name_to_pitch("invalid"), None);
    }

    #[test]
    fn test_note_overlaps() {
        let note = Note::new(60, 100, 0.0, 1.0);
        assert!(note.overlaps(0.5, 1.5));
        assert!(!note.overlaps(1.0, 2.0));
        assert!(note.overlaps(-0.5, 0.5));
    }
}
