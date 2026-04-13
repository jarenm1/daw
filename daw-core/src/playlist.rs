//! Playlist track data model
//!
//! Playlist tracks are horizontal lanes in the arrangement view.

use crate::{clip::PlaylistClip, BeatTime, Id};
use serde::{Deserialize, Serialize};

/// Unique identifier for a track
pub type TrackId = Id;

/// Playlist track (horizontal lane in the arrangement)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PlaylistTrack {
    pub id: TrackId,
    pub name: String,
    /// Color for UI representation
    pub color: String,
    /// Clips on this track
    pub clips: Vec<PlaylistClip>,
    /// Mute state
    pub muted: bool,
    /// Solo state
    pub solo: bool,
    /// Track height (for UI)
    pub height: u32,
    /// Track index (for ordering)
    pub index: usize,
}

impl PlaylistTrack {
    /// Create a new playlist track
    pub fn new(id: TrackId, name: impl Into<String>, color: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            color: color.into(),
            clips: Vec::new(),
            muted: false,
            solo: false,
            height: 64, // Default height in pixels
            index: id,
        }
    }

    /// Add a clip to this track
    pub fn add_clip(
        &mut self,
        pattern_id: usize,
        start_beat: BeatTime,
        duration_beats: BeatTime,
    ) -> usize {
        let id = self.clips.len();
        self.clips.push(PlaylistClip::new(
            id,
            pattern_id,
            self.id,
            start_beat,
            duration_beats,
        ));
        id
    }

    /// Get a clip by ID
    pub fn get_clip(&self, clip_id: usize) -> Option<&PlaylistClip> {
        self.clips.iter().find(|c| c.id == clip_id)
    }

    /// Get a mutable clip by ID
    pub fn get_clip_mut(&mut self, clip_id: usize) -> Option<&mut PlaylistClip> {
        self.clips.iter_mut().find(|c| c.id == clip_id)
    }

    /// Remove a clip by ID
    pub fn remove_clip(&mut self, clip_id: usize) -> Option<PlaylistClip> {
        if let Some(index) = self.clips.iter().position(|c| c.id == clip_id) {
            Some(self.clips.remove(index))
        } else {
            None
        }
    }

    /// Get clips at a given beat position
    pub fn clips_at_beat(&self, beat: BeatTime) -> Vec<&PlaylistClip> {
        self.clips
            .iter()
            .filter(|clip| clip.contains(beat) && !clip.muted)
            .collect()
    }

    /// Get clips in a time range
    pub fn clips_in_range(&self, start: BeatTime, end: BeatTime) -> Vec<&PlaylistClip> {
        self.clips
            .iter()
            .filter(|clip| clip.overlaps(start, end))
            .collect()
    }

    /// Get the last beat occupied by any clip
    pub fn end_beat(&self) -> BeatTime {
        self.clips
            .iter()
            .map(|clip| clip.end_beat())
            .fold(0.0, f64::max)
    }

    /// Toggle mute state
    pub fn toggle_mute(&mut self) -> bool {
        self.muted = !self.muted;
        self.muted
    }

    /// Toggle solo state
    pub fn toggle_solo(&mut self) -> bool {
        self.solo = !self.solo;
        self.solo
    }

    /// Check if track is effectively muted (considering solo states)
    pub fn is_effective_mute(&self, any_solo_active: bool) -> bool {
        self.muted || (any_solo_active && !self.solo)
    }

    /// Reorder clips by start time (useful after editing)
    pub fn sort_clips(&mut self) {
        self.clips.sort_by(|a, b| {
            a.start_beat
                .partial_cmp(&b.start_beat)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Find clips that overlap with a given time range on this track
    pub fn find_overlapping_clips(
        &self,
        start: BeatTime,
        duration: BeatTime,
    ) -> Vec<&PlaylistClip> {
        let end = start + duration;
        self.clips_in_range(start, end)
    }

    /// Move a clip to a new position
    pub fn move_clip(&mut self, clip_id: usize, new_start: BeatTime) -> Option<()> {
        if let Some(clip) = self.get_clip_mut(clip_id) {
            clip.move_to(new_start);
            Some(())
        } else {
            None
        }
    }
}

impl Default for PlaylistTrack {
    fn default() -> Self {
        Self::new(0, "Track 1", "#3b82f6")
    }
}

/// Helper struct for track ordering
#[derive(Clone, Debug)]
pub struct TrackOrder {
    pub track_ids: Vec<TrackId>,
}

impl TrackOrder {
    pub fn new() -> Self {
        Self {
            track_ids: Vec::new(),
        }
    }

    pub fn add_track(&mut self, track_id: TrackId) {
        if !self.track_ids.contains(&track_id) {
            self.track_ids.push(track_id);
        }
    }

    pub fn remove_track(&mut self, track_id: TrackId) {
        self.track_ids.retain(|&id| id != track_id);
    }

    pub fn move_track(&mut self, track_id: TrackId, new_index: usize) {
        if let Some(current_index) = self.track_ids.iter().position(|&id| id == track_id) {
            let id = self.track_ids.remove(current_index);
            let new_index = new_index.min(self.track_ids.len());
            self.track_ids.insert(new_index, id);
        }
    }

    pub fn index_of(&self, track_id: TrackId) -> Option<usize> {
        self.track_ids.iter().position(|&id| id == track_id)
    }
}

impl Default for TrackOrder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_new() {
        let track = PlaylistTrack::new(0, "Drums", "#ff0000");
        assert_eq!(track.name, "Drums");
        assert!(track.clips.is_empty());
    }

    #[test]
    fn test_add_clip() {
        let mut track = PlaylistTrack::new(0, "Test", "#000");
        let clip_id = track.add_clip(1, 0.0, 4.0);
        assert_eq!(clip_id, 0);
        assert_eq!(track.clips.len(), 1);
    }

    #[test]
    fn test_clips_at_beat() {
        let mut track = PlaylistTrack::new(0, "Test", "#000");
        track.add_clip(1, 0.0, 4.0);
        track.add_clip(2, 8.0, 4.0);

        let clips = track.clips_at_beat(2.0);
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].pattern_id, 1);
    }

    #[test]
    fn test_track_order() {
        let mut order = TrackOrder::new();
        order.add_track(0);
        order.add_track(1);
        order.add_track(2);

        order.move_track(2, 0);
        assert_eq!(order.track_ids, vec![2, 0, 1]);
    }
}
