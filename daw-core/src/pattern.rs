//! Pattern data model
//!
//! A pattern contains note data for step sequencing or piano roll.
//! This is the fundamental unit of musical content in the DAW.

use crate::{note::Note, BeatTime, ChannelId, Id};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for a pattern
pub type PatternId = Id;

/// A pattern contains note data for step sequencing or piano roll
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Pattern {
    pub id: PatternId,
    pub name: String,
    /// Color for UI representation (hex string like "#3b82f6")
    pub color: String,
    /// Notes for this pattern (pitch, velocity, start_beat, duration_beats)
    pub notes: Vec<Note>,
    /// Step sequencer data: 16 or 32 steps per channel
    /// channel_id -> step on/off
    pub step_data: HashMap<ChannelId, Vec<bool>>,
    /// Length of pattern in beats
    pub length_beats: BeatTime,
    /// Default number of steps for step sequencer
    pub step_count: usize,
}

impl Pattern {
    /// Create a new pattern with default settings
    pub fn new(id: PatternId, name: impl Into<String>, color: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            color: color.into(),
            notes: Vec::new(),
            step_data: HashMap::new(),
            length_beats: 4.0, // Default 4 beats (1 bar in 4/4)
            step_count: 16,
        }
    }

    /// Add a note to the pattern
    pub fn add_note(
        &mut self,
        pitch: u8,
        velocity: u8,
        start: BeatTime,
        duration: BeatTime,
    ) -> &mut Note {
        self.notes.push(Note {
            pitch,
            velocity,
            start_beat: start,
            duration_beats: duration,
        });
        self.notes.last_mut().unwrap()
    }

    /// Remove a note by index
    pub fn remove_note(&mut self, index: usize) -> Option<Note> {
        if index < self.notes.len() {
            Some(self.notes.remove(index))
        } else {
            None
        }
    }

    /// Get notes within a time range
    pub fn notes_in_range(&self, start: BeatTime, end: BeatTime) -> Vec<&Note> {
        self.notes
            .iter()
            .filter(|note| {
                let note_end = note.start_beat + note.duration_beats;
                note.start_beat < end && note_end > start
            })
            .collect()
    }

    /// Toggle a step for a specific channel
    /// Returns the new state of the step
    pub fn toggle_step(&mut self, channel_id: ChannelId, step: usize) -> bool {
        let steps = self
            .step_data
            .entry(channel_id)
            .or_insert_with(|| vec![false; self.step_count]);

        if step < steps.len() {
            steps[step] = !steps[step];
            steps[step]
        } else {
            false
        }
    }

    /// Set step state for a specific channel
    pub fn set_step(&mut self, channel_id: ChannelId, step: usize, active: bool) {
        let steps = self
            .step_data
            .entry(channel_id)
            .or_insert_with(|| vec![false; self.step_count]);

        if step < steps.len() {
            steps[step] = active;
        }
    }

    /// Check if a step is active
    pub fn is_step_active(&self, channel_id: ChannelId, step: usize) -> bool {
        self.step_data
            .get(&channel_id)
            .and_then(|steps| steps.get(step))
            .copied()
            .unwrap_or(false)
    }

    /// Get step data for a channel, initializing if needed
    pub fn get_or_create_steps(&mut self, channel_id: ChannelId) -> &mut Vec<bool> {
        self.step_data
            .entry(channel_id)
            .or_insert_with(|| vec![false; self.step_count])
    }

    /// Clear all notes
    pub fn clear_notes(&mut self) {
        self.notes.clear();
    }

    /// Clear all step data
    pub fn clear_steps(&mut self) {
        self.step_data.clear();
    }

    /// Get the note at a specific beat position (if any)
    pub fn note_at(&self, beat: BeatTime, tolerance: BeatTime) -> Option<&Note> {
        self.notes
            .iter()
            .find(|note| (note.start_beat - beat).abs() < tolerance)
    }

    /// Transpose all notes by a number of semitones
    pub fn transpose(&mut self, semitones: i8) {
        for note in &mut self.notes {
            let new_pitch = (note.pitch as i16 + semitones as i16).clamp(0, 127);
            note.pitch = new_pitch as u8;
        }
    }

    /// Quantize note starts to a grid
    pub fn quantize(&mut self, grid_division: BeatTime) {
        for note in &mut self.notes {
            note.start_beat = (note.start_beat / grid_division).round() * grid_division;
        }
    }

    /// Resize pattern length while optionally scaling notes
    pub fn resize(&mut self, new_length: BeatTime, scale_notes: bool) {
        if scale_notes && self.length_beats > 0.0 {
            let scale = new_length / self.length_beats;
            for note in &mut self.notes {
                note.start_beat *= scale;
                note.duration_beats *= scale;
            }
        }
        self.length_beats = new_length;
    }
}

impl Default for Pattern {
    fn default() -> Self {
        Self::new(0, "Pattern 1", "#3b82f6")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_new() {
        let pattern = Pattern::new(1, "Test", "#ff0000");
        assert_eq!(pattern.id, 1);
        assert_eq!(pattern.name, "Test");
        assert_eq!(pattern.length_beats, 4.0);
        assert!(pattern.notes.is_empty());
    }

    #[test]
    fn test_add_note() {
        let mut pattern = Pattern::new(0, "Test", "#000");
        pattern.add_note(60, 100, 0.0, 1.0);
        assert_eq!(pattern.notes.len(), 1);
        assert_eq!(pattern.notes[0].pitch, 60);
    }

    #[test]
    fn test_toggle_step() {
        let mut pattern = Pattern::new(0, "Test", "#000");
        let channel_id = 0;

        assert!(!pattern.is_step_active(channel_id, 0));
        assert!(pattern.toggle_step(channel_id, 0));
        assert!(pattern.is_step_active(channel_id, 0));
        assert!(!pattern.toggle_step(channel_id, 0));
        assert!(!pattern.is_step_active(channel_id, 0));
    }

    #[test]
    fn test_transpose() {
        let mut pattern = Pattern::new(0, "Test", "#000");
        pattern.add_note(60, 100, 0.0, 1.0);
        pattern.transpose(12);
        assert_eq!(pattern.notes[0].pitch, 72);
    }

    #[test]
    fn test_quantize() {
        let mut pattern = Pattern::new(0, "Test", "#000");
        pattern.add_note(60, 100, 0.25, 0.5); // Slightly off-grid
        pattern.quantize(0.25);
        assert_eq!(pattern.notes[0].start_beat, 0.25);
    }
}
