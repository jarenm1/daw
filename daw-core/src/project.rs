//! Project data model
//!
//! The Project is the root container for all DAW data.

use crate::{
    channel::{Channel, ChannelId},
    clip::PlaylistClip,
    pattern::{Pattern, PatternId},
    playlist::{PlaylistTrack, TrackId},
    BeatTime, Id,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for a project
pub type ProjectId = Id;

/// Playback mode: Pattern mode or Song mode
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum PlaybackMode {
    /// Play the selected pattern in a loop
    #[default]
    Pattern,
    /// Play the full playlist arrangement
    Song,
}

/// The complete project state
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Project {
    /// Project ID
    pub id: ProjectId,
    /// Project name
    pub name: String,
    /// All patterns available in the project
    pub patterns: HashMap<PatternId, Pattern>,
    /// Currently selected pattern for editing (UI state)
    #[serde(skip)]
    pub selected_pattern_id: Option<PatternId>,
    /// Currently active pattern for pattern mode playback
    pub current_pattern_id: PatternId,
    /// Channel rack channels
    pub channels: Vec<Channel>,
    /// Playlist tracks
    pub playlist_tracks: Vec<PlaylistTrack>,
    /// Current playback mode
    pub playback_mode: PlaybackMode,
    /// Global BPM
    pub bpm: f64,
    /// Time signature
    pub time_sig_num: u8,
    pub time_sig_denom: u8,
    /// Ticks per beat for time display (FL Studio uses 960)
    pub ticks_per_beat: u16,
    /// Project file path (if saved)
    pub file_path: Option<String>,
    /// Whether project has unsaved changes
    #[serde(skip)]
    pub dirty: bool,
    /// Project version for migration
    pub version: u32,
}

impl Project {
    /// Create a new empty project
    pub fn new(id: ProjectId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            patterns: HashMap::new(),
            selected_pattern_id: None,
            current_pattern_id: 0,
            channels: Vec::new(),
            playlist_tracks: Vec::new(),
            playback_mode: PlaybackMode::Pattern,
            bpm: 120.0,
            time_sig_num: 4,
            time_sig_denom: 4,
            ticks_per_beat: 960,
            file_path: None,
            dirty: true,
            version: 1,
        }
    }

    /// Create a project with default content (patterns, channels, tracks)
    pub fn with_defaults(id: ProjectId, name: impl Into<String>) -> Self {
        let mut project = Self::new(id, name);

        // Create default pattern 1
        let pattern1 = Pattern::new(0, "Pattern 1", "#3b82f6");
        project.patterns.insert(0, pattern1);
        project.selected_pattern_id = Some(0);

        // Create default channels (FL Studio style)
        let channel_colors = [
            "#ef4444", "#f97316", "#fbbf24", "#4ade80", "#3b82f6", "#a855f7", "#ec4899", "#22d3ee",
        ];
        let channel_names = [
            "Kick", "Snare", "HiHat", "Clap", "Bass", "Lead", "Pad", "FX",
        ];

        for (i, (name, color)) in channel_names.iter().zip(channel_colors.iter()).enumerate() {
            project
                .channels
                .push(Channel::new_synth(i, name.to_string(), color.to_string()));
        }

        // Create playlist tracks
        let track_colors = [
            "#3b82f6", "#ef4444", "#4ade80", "#fbbf24", "#a855f7", "#f97316", "#22d3ee", "#ec4899",
        ];
        for (i, color) in track_colors.iter().enumerate() {
            project.playlist_tracks.push(PlaylistTrack::new(
                i,
                format!("Track {}", i + 1),
                color.to_string(),
            ));
        }

        project
    }

    // === Pattern Operations ===

    /// Get a pattern by ID
    pub fn get_pattern(&self, id: PatternId) -> Option<&Pattern> {
        self.patterns.get(&id)
    }

    /// Get a mutable pattern by ID
    pub fn get_pattern_mut(&mut self, id: PatternId) -> Option<&mut Pattern> {
        self.patterns.get_mut(&id)
    }

    /// Create a new pattern
    pub fn create_pattern(
        &mut self,
        name: impl Into<String>,
        color: impl Into<String>,
    ) -> PatternId {
        let id = self.next_pattern_id();
        self.patterns.insert(id, Pattern::new(id, name, color));
        self.mark_dirty();
        id
    }

    /// Delete a pattern
    pub fn delete_pattern(&mut self, id: PatternId) -> Option<Pattern> {
        let removed = self.patterns.remove(&id);
        if removed.is_some() {
            self.mark_dirty();
        }
        removed
    }

    /// Duplicate a pattern
    pub fn duplicate_pattern(&mut self, id: PatternId) -> Option<PatternId> {
        let pattern = self.get_pattern(id)?.clone();
        let new_id = self.next_pattern_id();
        let mut new_pattern = pattern;
        new_pattern.id = new_id;
        new_pattern.name = format!("{} Copy", new_pattern.name);
        self.patterns.insert(new_id, new_pattern);
        self.mark_dirty();
        Some(new_id)
    }

    /// Get the currently selected pattern
    pub fn get_selected_pattern(&self) -> Option<&Pattern> {
        self.selected_pattern_id
            .and_then(|id| self.patterns.get(&id))
    }

    /// Get the current pattern (for pattern mode playback)
    pub fn get_current_pattern(&self) -> Option<&Pattern> {
        self.patterns.get(&self.current_pattern_id)
    }

    /// Set the current pattern
    pub fn set_current_pattern(&mut self, id: PatternId) {
        if self.patterns.contains_key(&id) {
            self.current_pattern_id = id;
        }
    }

    /// Get next available pattern ID
    fn next_pattern_id(&self) -> PatternId {
        self.patterns
            .keys()
            .copied()
            .max()
            .map(|m| m + 1)
            .unwrap_or(0)
    }

    // === Channel Operations ===

    /// Get a channel by ID
    pub fn get_channel(&self, id: ChannelId) -> Option<&Channel> {
        self.channels.get(id)
    }

    /// Get a mutable channel by ID
    pub fn get_channel_mut(&mut self, id: ChannelId) -> Option<&mut Channel> {
        self.channels.get_mut(id)
    }

    /// Add a channel
    pub fn add_channel(&mut self, channel: Channel) -> ChannelId {
        let id = self.channels.len();
        let mut channel = channel;
        channel.id = id;
        self.channels.push(channel);
        self.mark_dirty();
        id
    }

    /// Remove a channel
    pub fn remove_channel(&mut self, id: ChannelId) -> Option<Channel> {
        if id < self.channels.len() {
            self.mark_dirty();
            Some(self.channels.remove(id))
        } else {
            None
        }
    }

    /// Check if any channel is soloed
    pub fn any_solo_active(&self) -> bool {
        self.channels.iter().any(|c| c.solo)
    }

    // === Playlist Operations ===

    /// Get a track by ID
    pub fn get_track(&self, id: TrackId) -> Option<&PlaylistTrack> {
        self.playlist_tracks.get(id)
    }

    /// Get a mutable track by ID
    pub fn get_track_mut(&mut self, id: TrackId) -> Option<&mut PlaylistTrack> {
        self.playlist_tracks.get_mut(id)
    }

    /// Add a track
    pub fn add_track(&mut self, track: PlaylistTrack) -> TrackId {
        let id = self.playlist_tracks.len();
        let mut track = track;
        track.id = id;
        track.index = id;
        self.playlist_tracks.push(track);
        self.mark_dirty();
        id
    }

    /// Remove a track
    pub fn remove_track(&mut self, id: TrackId) -> Option<PlaylistTrack> {
        if id < self.playlist_tracks.len() {
            self.mark_dirty();
            Some(self.playlist_tracks.remove(id))
        } else {
            None
        }
    }

    /// Get clips at a given beat position across all tracks
    pub fn get_clips_at_beat(&self, beat: BeatTime) -> Vec<&PlaylistClip> {
        let mut result = Vec::new();
        for track in &self.playlist_tracks {
            if track.is_effective_mute(self.playlist_tracks.iter().any(|t| t.solo)) {
                continue;
            }
            for clip in &track.clips {
                if !clip.muted && clip.contains(beat) {
                    result.push(clip);
                }
            }
        }
        result
    }

    /// Get the total duration of the project (last clip end)
    pub fn duration_beats(&self) -> BeatTime {
        self.playlist_tracks
            .iter()
            .map(|track| track.end_beat())
            .fold(0.0, f64::max)
    }

    // === Utility ===

    /// Mark project as having unsaved changes
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Mark project as saved
    pub fn mark_saved(&mut self, path: Option<String>) {
        self.dirty = false;
        self.file_path = path;
    }

    /// Set BPM
    pub fn set_bpm(&mut self, bpm: f64) {
        self.bpm = bpm.clamp(20.0, 999.0);
        self.mark_dirty();
    }

    /// Format current time as bars:beats:ticks
    pub fn format_time(&self, beat: BeatTime) -> String {
        crate::format_time_bbt(
            beat,
            self.time_sig_num,
            self.time_sig_denom,
            self.ticks_per_beat,
        )
    }

    /// Toggle playback mode
    pub fn toggle_playback_mode(&mut self) {
        self.playback_mode = match self.playback_mode {
            PlaybackMode::Pattern => PlaybackMode::Song,
            PlaybackMode::Song => PlaybackMode::Pattern,
        };
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl Default for Project {
    fn default() -> Self {
        Self::with_defaults(0, "Untitled")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_new() {
        let project = Project::new(1, "Test Project");
        assert_eq!(project.name, "Test Project");
        assert!(project.patterns.is_empty());
        assert_eq!(project.bpm, 120.0);
    }

    #[test]
    fn test_project_with_defaults() {
        let project = Project::with_defaults(0, "Test");
        assert!(!project.patterns.is_empty());
        assert!(!project.channels.is_empty());
        assert!(!project.playlist_tracks.is_empty());
    }

    #[test]
    fn test_create_pattern() {
        let mut project = Project::new(0, "Test");
        let id = project.create_pattern("Bass", "#ff0000");
        assert!(project.patterns.contains_key(&id));
        assert_eq!(project.patterns[&id].name, "Bass");
    }

    #[test]
    fn test_duplicate_pattern() {
        let mut project = Project::with_defaults(0, "Test");
        let original_id = project.current_pattern_id;
        let new_id = project.duplicate_pattern(original_id);
        assert!(new_id.is_some());
        assert!(project.patterns.contains_key(&new_id.unwrap()));
    }

    #[test]
    fn test_serialization() {
        let project = Project::with_defaults(0, "Test");
        let json = project.to_json().unwrap();
        let deserialized = Project::from_json(&json).unwrap();
        assert_eq!(project.name, deserialized.name);
        assert_eq!(project.patterns.len(), deserialized.patterns.len());
    }

    #[test]
    fn test_format_time() {
        let project = Project::with_defaults(0, "Test");
        assert_eq!(project.format_time(0.0), "001:01:000");
        assert_eq!(project.format_time(4.0), "002:01:000");
    }
}
