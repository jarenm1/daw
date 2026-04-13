//! Clip data model
//!
//! Clips are references to patterns placed at specific time positions.

use crate::{pattern::PatternId, BeatTime, Id};
use serde::{Deserialize, Serialize};

/// Unique identifier for a clip
pub type ClipId = Id;

/// A clip in the playlist - references a pattern placed at a time position
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PlaylistClip {
    pub id: ClipId,
    /// Pattern that this clip plays
    pub pattern_id: PatternId,
    /// Track index in the playlist
    pub track_index: usize,
    /// When the clip starts in the playlist (in beats)
    pub start_beat: BeatTime,
    /// How long it plays (in beats)
    pub duration_beats: BeatTime,
    /// If true, pattern loops to fill duration
    pub looped: bool,
    /// Color override (None = use pattern color)
    pub color: Option<String>,
    /// Clip name override (None = use pattern name)
    pub name: Option<String>,
    /// Mute state
    pub muted: bool,
}

impl PlaylistClip {
    /// Create a new playlist clip
    pub fn new(
        id: ClipId,
        pattern_id: PatternId,
        track_index: usize,
        start_beat: BeatTime,
        duration_beats: BeatTime,
    ) -> Self {
        Self {
            id,
            pattern_id,
            track_index,
            start_beat,
            duration_beats,
            looped: true,
            color: None,
            name: None,
            muted: false,
        }
    }

    /// Get the end position of this clip
    pub fn end_beat(&self) -> BeatTime {
        self.start_beat + self.duration_beats
    }

    /// Check if this clip contains a given beat position
    pub fn contains(&self, beat: BeatTime) -> bool {
        beat >= self.start_beat && beat < self.end_beat()
    }

    /// Check if this clip overlaps with a time range
    pub fn overlaps(&self, start: BeatTime, end: BeatTime) -> bool {
        self.start_beat < end && self.end_beat() > start
    }

    /// Get the pattern-relative beat for a playlist beat
    /// Returns None if the playlist beat is outside this clip
    pub fn pattern_beat(&self, playlist_beat: BeatTime) -> Option<BeatTime> {
        if !self.contains(playlist_beat) {
            return None;
        }

        let offset = playlist_beat - self.start_beat;

        if self.looped {
            // Get pattern length from pattern - placeholder for now
            // In real implementation, we'd need access to the pattern
            Some(offset % self.duration_beats)
        } else {
            Some(offset)
        }
    }

    /// Move the clip to a new start position
    pub fn move_to(&mut self, new_start: BeatTime) {
        self.start_beat = new_start;
    }

    /// Resize the clip duration
    pub fn resize(&mut self, new_duration: BeatTime) {
        self.duration_beats = new_duration.max(0.0);
    }

    /// Toggle loop state
    pub fn toggle_loop(&mut self) -> bool {
        self.looped = !self.looped;
        self.looped
    }

    /// Toggle mute state
    pub fn toggle_mute(&mut self) -> bool {
        self.muted = !self.muted;
        self.muted
    }
}

/// A reference to a clip occurrence (for selection/editing)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ClipOccurrence {
    pub clip_id: ClipId,
    pub pattern_id: PatternId,
    pub start_beat: BeatTime,
    pub track_index: usize,
}

/// Types of clips
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Clip {
    /// Pattern-based clip (MIDI/notes)
    Pattern(PlaylistClip),
    /// Automation clip
    Automation(PlaylistClip),
    /// Audio clip (direct audio file)
    Audio(AudioClip),
}

/// Audio clip data
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AudioClip {
    pub id: ClipId,
    pub file_path: String,
    pub track_index: usize,
    pub start_beat: BeatTime,
    pub duration_beats: BeatTime,
    /// Offset into the audio file to start playing from
    pub file_offset_seconds: f64,
    /// Time-stretch ratio
    pub time_stretch: f64,
    pub muted: bool,
}

impl AudioClip {
    /// Create a new audio clip
    pub fn new(
        id: ClipId,
        file_path: impl Into<String>,
        track_index: usize,
        start_beat: BeatTime,
        duration_beats: BeatTime,
    ) -> Self {
        Self {
            id,
            file_path: file_path.into(),
            track_index,
            start_beat,
            duration_beats,
            file_offset_seconds: 0.0,
            time_stretch: 1.0,
            muted: false,
        }
    }

    /// Get the end position
    pub fn end_beat(&self) -> BeatTime {
        self.start_beat + self.duration_beats
    }

    /// Check if this clip contains a given beat position
    pub fn contains(&self, beat: BeatTime) -> bool {
        beat >= self.start_beat && beat < self.end_beat()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playlist_clip_new() {
        let clip = PlaylistClip::new(0, 1, 2, 4.0, 8.0);
        assert_eq!(clip.pattern_id, 1);
        assert_eq!(clip.track_index, 2);
        assert_eq!(clip.start_beat, 4.0);
        assert_eq!(clip.duration_beats, 8.0);
        assert!(clip.looped);
    }

    #[test]
    fn test_clip_contains() {
        let clip = PlaylistClip::new(0, 1, 0, 4.0, 4.0);
        assert!(clip.contains(4.0));
        assert!(clip.contains(7.9));
        assert!(!clip.contains(8.0));
        assert!(!clip.contains(3.9));
    }

    #[test]
    fn test_pattern_beat_looped() {
        let mut clip = PlaylistClip::new(0, 1, 0, 0.0, 4.0);
        clip.looped = true;

        // Should loop every 4 beats
        assert_eq!(clip.pattern_beat(0.0), Some(0.0));
        assert_eq!(clip.pattern_beat(2.0), Some(2.0));
        assert_eq!(clip.pattern_beat(4.0), Some(0.0));
        assert_eq!(clip.pattern_beat(5.0), Some(1.0));
        assert_eq!(clip.pattern_beat(10.0), Some(2.0));
    }

    #[test]
    fn test_pattern_beat_not_looped() {
        let mut clip = PlaylistClip::new(0, 1, 0, 0.0, 4.0);
        clip.looped = false;

        // Should return linear offset
        assert_eq!(clip.pattern_beat(0.0), Some(0.0));
        assert_eq!(clip.pattern_beat(2.0), Some(2.0));
        assert_eq!(clip.pattern_beat(5.0), Some(5.0));
    }

    #[test]
    fn test_overlaps() {
        let clip = PlaylistClip::new(0, 1, 0, 4.0, 4.0); // 4.0 - 8.0
        assert!(clip.overlaps(0.0, 5.0)); // overlaps at start
        assert!(clip.overlaps(5.0, 10.0)); // overlaps at end
        assert!(clip.overlaps(5.0, 6.0)); // contained within
        assert!(!clip.overlaps(0.0, 4.0)); // touches at start but doesn't overlap
        assert!(!clip.overlaps(8.0, 10.0)); // touches at end but doesn't overlap
    }
}
