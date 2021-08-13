//! Core data models for the DAW
//!
//! This crate contains pure data structures and operations for:
//! - Projects (patterns, channels, playlist)
//! - Timeline operations
//! - Serialization/deserialization
//!
//! No UI or audio dependencies - completely headless.

pub mod channel;
pub mod clip;
pub mod note;
pub mod pattern;
pub mod playlist;
pub mod project;

pub use channel::{Channel, ChannelId, ChannelType};
pub use clip::{Clip, ClipId, PlaylistClip};
pub use note::Note;
pub use pattern::{Pattern, PatternId};
pub use playlist::{PlaylistTrack, TrackId};
pub use project::{PlaybackMode, Project, ProjectId};

/// Unique ID generator for all entities
pub type Id = usize;

/// Time position in beats (musical time)
pub type BeatTime = f64;

/// Sample-accurate time (for audio engine)
pub type SampleTime = i64;

/// Convert beats to seconds at a given BPM
pub fn beats_to_seconds(beats: BeatTime, bpm: f64) -> f64 {
    beats * 60.0 / bpm
}

/// Convert seconds to beats at a given BPM
pub fn seconds_to_beats(seconds: f64, bpm: f64) -> BeatTime {
    seconds * bpm / 60.0
}

/// Format time as bars:beats:ticks (FL Studio style)
/// - ticks_per_beat: typically 960 or 480
pub fn format_time_bbt(
    beats: BeatTime,
    time_sig_num: u8,
    time_sig_denom: u8,
    ticks_per_beat: u16,
) -> String {
    let total_beats = beats * (4.0 / time_sig_denom as f64);
    let bars = (total_beats / time_sig_num as f64).floor() as i32 + 1;
    let beat_in_bar = (total_beats % time_sig_num as f64).floor() as i32 + 1;
    let ticks = ((beats.fract()) * ticks_per_beat as f64) as i32;

    format!("{:03}:{:02}:{:03}", bars, beat_in_bar, ticks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beats_seconds_conversion() {
        assert_eq!(beats_to_seconds(1.0, 120.0), 0.5);
        assert_eq!(beats_to_seconds(4.0, 120.0), 2.0);
        assert_eq!(seconds_to_beats(0.5, 120.0), 1.0);
        assert_eq!(seconds_to_beats(2.0, 120.0), 4.0);
    }

    #[test]
    fn test_format_time_bbt() {
        // 4/4 time, 960 ticks per beat
        assert_eq!(format_time_bbt(0.0, 4, 4, 960), "001:01:000");
        assert_eq!(format_time_bbt(4.0, 4, 4, 960), "002:01:000");
        assert_eq!(format_time_bbt(5.5, 4, 4, 960), "002:02:480");
    }
}
