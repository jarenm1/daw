//! Built-in DAW commands
//!
//! These commands operate on the Project and can be dispatched from any frontend.

use crate::Command;
use anyhow::Result;
use daw_core::{
    channel::Channel, playlist::PlaylistTrack, BeatTime, ChannelId, PatternId, Project, TrackId,
};
use serde::Deserialize;

// === Pattern Commands ===

/// Create a new pattern
#[derive(Debug, Clone)]
pub struct CreatePattern {
    pub name: String,
    pub color: String,
}

impl Command for CreatePattern {
    fn execute(&self, project: &mut Project) -> Result<bool> {
        let _id = project.create_pattern(&self.name, &self.color);
        Ok(true)
    }

    fn name(&self) -> &str {
        "create_pattern"
    }
}

/// Delete a pattern
#[derive(Debug, Clone)]
pub struct DeletePattern {
    pub pattern_id: PatternId,
}

impl Command for DeletePattern {
    fn execute(&self, project: &mut Project) -> Result<bool> {
        if project.delete_pattern(self.pattern_id).is_some() {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn name(&self) -> &str {
        "delete_pattern"
    }
}

/// Duplicate a pattern
#[derive(Debug, Clone)]
pub struct DuplicatePattern {
    pub pattern_id: PatternId,
}

impl Command for DuplicatePattern {
    fn execute(&self, project: &mut Project) -> Result<bool> {
        if project.duplicate_pattern(self.pattern_id).is_some() {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn name(&self) -> &str {
        "duplicate_pattern"
    }
}

/// Set the current pattern (for pattern mode)
#[derive(Debug, Clone)]
pub struct SetCurrentPattern {
    pub pattern_id: PatternId,
}

impl Command for SetCurrentPattern {
    fn execute(&self, project: &mut Project) -> Result<bool> {
        let old_id = project.current_pattern_id;
        project.set_current_pattern(self.pattern_id);
        Ok(project.current_pattern_id != old_id)
    }

    fn name(&self) -> &str {
        "set_current_pattern"
    }
}

/// Add a note to a pattern
#[derive(Debug, Clone)]
pub struct AddNote {
    pub pattern_id: PatternId,
    pub pitch: u8,
    pub velocity: u8,
    pub start_beat: BeatTime,
    pub duration_beats: BeatTime,
}

impl Command for AddNote {
    fn execute(&self, project: &mut Project) -> Result<bool> {
        if let Some(pattern) = project.get_pattern_mut(self.pattern_id) {
            pattern.add_note(
                self.pitch,
                self.velocity,
                self.start_beat,
                self.duration_beats,
            );
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn name(&self) -> &str {
        "add_note"
    }
}

/// Toggle a step in the step sequencer
#[derive(Debug, Clone)]
pub struct ToggleStep {
    pub pattern_id: PatternId,
    pub channel_id: ChannelId,
    pub step: usize,
}

impl Command for ToggleStep {
    fn execute(&self, project: &mut Project) -> Result<bool> {
        if let Some(pattern) = project.get_pattern_mut(self.pattern_id) {
            pattern.toggle_step(self.channel_id, self.step);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn name(&self) -> &str {
        "toggle_step"
    }
}

/// Transpose a pattern
#[derive(Debug, Clone)]
pub struct TransposePattern {
    pub pattern_id: PatternId,
    pub semitones: i8,
}

impl Command for TransposePattern {
    fn execute(&self, project: &mut Project) -> Result<bool> {
        if let Some(pattern) = project.get_pattern_mut(self.pattern_id) {
            pattern.transpose(self.semitones);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn name(&self) -> &str {
        "transpose_pattern"
    }
}

// === Channel Commands ===

/// Add a synthesizer channel
#[derive(Debug, Clone)]
pub struct AddSynthChannel {
    pub name: String,
    pub color: String,
}

impl Command for AddSynthChannel {
    fn execute(&self, project: &mut Project) -> Result<bool> {
        let id = project.channels.len();
        let channel = Channel::new_synth(id, &self.name, &self.color);
        project.add_channel(channel);
        Ok(true)
    }

    fn name(&self) -> &str {
        "add_synth_channel"
    }
}

/// Add a sampler channel
#[derive(Debug, Clone)]
pub struct AddSamplerChannel {
    pub name: String,
    pub color: String,
    pub sample_path: Option<String>,
}

impl Command for AddSamplerChannel {
    fn execute(&self, project: &mut Project) -> Result<bool> {
        let id = project.channels.len();
        let channel = Channel::new_sampler(id, &self.name, &self.color, self.sample_path.clone());
        project.add_channel(channel);
        Ok(true)
    }

    fn name(&self) -> &str {
        "add_sampler_channel"
    }
}

/// Remove a channel
#[derive(Debug, Clone)]
pub struct RemoveChannel {
    pub channel_id: ChannelId,
}

impl Command for RemoveChannel {
    fn execute(&self, project: &mut Project) -> Result<bool> {
        if project.remove_channel(self.channel_id).is_some() {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn name(&self) -> &str {
        "remove_channel"
    }
}

/// Set channel volume
#[derive(Debug, Clone)]
pub struct SetChannelVolume {
    pub channel_id: ChannelId,
    pub volume: f32,
}

impl Command for SetChannelVolume {
    fn execute(&self, project: &mut Project) -> Result<bool> {
        if let Some(channel) = project.get_channel_mut(self.channel_id) {
            channel.set_volume(self.volume);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn name(&self) -> &str {
        "set_channel_volume"
    }
}

/// Toggle channel mute
#[derive(Debug, Clone)]
pub struct ToggleChannelMute {
    pub channel_id: ChannelId,
}

impl Command for ToggleChannelMute {
    fn execute(&self, project: &mut Project) -> Result<bool> {
        if let Some(channel) = project.get_channel_mut(self.channel_id) {
            channel.toggle_mute();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn name(&self) -> &str {
        "toggle_channel_mute"
    }
}

/// Toggle channel solo
#[derive(Debug, Clone)]
pub struct ToggleChannelSolo {
    pub channel_id: ChannelId,
}

impl Command for ToggleChannelSolo {
    fn execute(&self, project: &mut Project) -> Result<bool> {
        if let Some(channel) = project.get_channel_mut(self.channel_id) {
            channel.toggle_solo();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn name(&self) -> &str {
        "toggle_channel_solo"
    }
}

// === Playlist Commands ===

/// Add a clip to a track
#[derive(Debug, Clone)]
pub struct AddClip {
    pub track_id: TrackId,
    pub pattern_id: PatternId,
    pub start_beat: BeatTime,
    pub duration_beats: BeatTime,
}

impl Command for AddClip {
    fn execute(&self, project: &mut Project) -> Result<bool> {
        if let Some(track) = project.get_track_mut(self.track_id) {
            track.add_clip(self.pattern_id, self.start_beat, self.duration_beats);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn name(&self) -> &str {
        "add_clip"
    }
}

/// Remove a clip from a track
#[derive(Debug, Clone)]
pub struct RemoveClip {
    pub track_id: TrackId,
    pub clip_id: usize,
}

impl Command for RemoveClip {
    fn execute(&self, project: &mut Project) -> Result<bool> {
        if let Some(track) = project.get_track_mut(self.track_id) {
            let removed = track.remove_clip(self.clip_id).is_some();
            Ok(removed)
        } else {
            Ok(false)
        }
    }

    fn name(&self) -> &str {
        "remove_clip"
    }
}

/// Move a clip to a new position
#[derive(Debug, Clone)]
pub struct MoveClip {
    pub track_id: TrackId,
    pub clip_id: usize,
    pub new_start_beat: BeatTime,
}

impl Command for MoveClip {
    fn execute(&self, project: &mut Project) -> Result<bool> {
        if let Some(track) = project.get_track_mut(self.track_id) {
            track.move_clip(self.clip_id, self.new_start_beat);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn name(&self) -> &str {
        "move_clip"
    }
}

/// Add a new track
#[derive(Debug, Clone)]
pub struct AddTrack {
    pub name: String,
    pub color: String,
}

impl Command for AddTrack {
    fn execute(&self, project: &mut Project) -> Result<bool> {
        let track = PlaylistTrack::new(0, &self.name, &self.color);
        project.add_track(track);
        Ok(true)
    }

    fn name(&self) -> &str {
        "add_track"
    }
}

/// Remove a track
#[derive(Debug, Clone)]
pub struct RemoveTrack {
    pub track_id: TrackId,
}

impl Command for RemoveTrack {
    fn execute(&self, project: &mut Project) -> Result<bool> {
        if project.remove_track(self.track_id).is_some() {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn name(&self) -> &str {
        "remove_track"
    }
}

// === Transport/Project Commands ===

/// Set the project BPM
#[derive(Debug, Clone)]
pub struct SetBpm {
    pub bpm: f64,
}

impl Command for SetBpm {
    fn execute(&self, project: &mut Project) -> Result<bool> {
        let old_bpm = project.bpm;
        project.set_bpm(self.bpm);
        Ok(project.bpm != old_bpm)
    }

    fn name(&self) -> &str {
        "set_bpm"
    }
}

/// Set the time signature
#[derive(Debug, Clone)]
pub struct SetTimeSignature {
    pub numerator: u8,
    pub denominator: u8,
}

impl Command for SetTimeSignature {
    fn execute(&self, project: &mut Project) -> Result<bool> {
        project.time_sig_num = self.numerator;
        project.time_sig_denom = self.denominator;
        Ok(true)
    }

    fn name(&self) -> &str {
        "set_time_signature"
    }
}

/// Toggle playback mode (Pattern/Song)
#[derive(Debug, Clone)]
pub struct TogglePlaybackMode;

impl Command for TogglePlaybackMode {
    fn execute(&self, project: &mut Project) -> Result<bool> {
        project.toggle_playback_mode();
        Ok(true)
    }

    fn name(&self) -> &str {
        "toggle_playback_mode"
    }
}

// === Command Factories ===

/// Factory function arguments for commands
#[derive(Debug, Deserialize)]
struct CreatePatternArgs {
    name: String,
    #[serde(default = "default_pattern_color")]
    color: String,
}

fn default_pattern_color() -> String {
    "#3b82f6".to_string()
}

/// Create a command from JSON arguments
pub fn create_pattern_from_json(args: &str) -> Result<Box<dyn Command>> {
    let parsed: CreatePatternArgs = serde_json::from_str(args)?;
    Ok(Box::new(CreatePattern {
        name: parsed.name,
        color: parsed.color,
    }))
}

/// Register all built-in commands with a dispatcher
pub fn register_default_commands(dispatcher: &mut crate::Dispatcher) {
    // Pattern commands
    dispatcher.register("create_pattern", |args| {
        let parsed: CreatePatternArgs = serde_json::from_str(args)?;
        Ok(Box::new(CreatePattern {
            name: parsed.name,
            color: parsed.color,
        }))
    });

    dispatcher.register("delete_pattern", |args| {
        #[derive(Deserialize)]
        struct Args {
            pattern_id: PatternId,
        }
        let parsed: Args = serde_json::from_str(args)?;
        Ok(Box::new(DeletePattern {
            pattern_id: parsed.pattern_id,
        }))
    });

    dispatcher.register("duplicate_pattern", |args| {
        #[derive(Deserialize)]
        struct Args {
            pattern_id: PatternId,
        }
        let parsed: Args = serde_json::from_str(args)?;
        Ok(Box::new(DuplicatePattern {
            pattern_id: parsed.pattern_id,
        }))
    });

    dispatcher.register("set_current_pattern", |args| {
        #[derive(Deserialize)]
        struct Args {
            pattern_id: PatternId,
        }
        let parsed: Args = serde_json::from_str(args)?;
        Ok(Box::new(SetCurrentPattern {
            pattern_id: parsed.pattern_id,
        }))
    });

    dispatcher.register("toggle_step", |args| {
        #[derive(Deserialize)]
        struct Args {
            pattern_id: PatternId,
            channel_id: ChannelId,
            step: usize,
        }
        let parsed: Args = serde_json::from_str(args)?;
        Ok(Box::new(ToggleStep {
            pattern_id: parsed.pattern_id,
            channel_id: parsed.channel_id,
            step: parsed.step,
        }))
    });

    dispatcher.register("transpose_pattern", |args| {
        #[derive(Deserialize)]
        struct Args {
            pattern_id: PatternId,
            semitones: i8,
        }
        let parsed: Args = serde_json::from_str(args)?;
        Ok(Box::new(TransposePattern {
            pattern_id: parsed.pattern_id,
            semitones: parsed.semitones,
        }))
    });

    // Channel commands
    dispatcher.register("add_synth_channel", |args| {
        #[derive(Deserialize)]
        struct Args {
            name: String,
            #[serde(default = "default_channel_color")]
            color: String,
        }
        fn default_channel_color() -> String {
            "#3b82f6".to_string()
        }
        let parsed: Args = serde_json::from_str(args)?;
        Ok(Box::new(AddSynthChannel {
            name: parsed.name,
            color: parsed.color,
        }))
    });

    dispatcher.register("remove_channel", |args| {
        #[derive(Deserialize)]
        struct Args {
            channel_id: ChannelId,
        }
        let parsed: Args = serde_json::from_str(args)?;
        Ok(Box::new(RemoveChannel {
            channel_id: parsed.channel_id,
        }))
    });

    dispatcher.register("toggle_channel_mute", |args| {
        #[derive(Deserialize)]
        struct Args {
            channel_id: ChannelId,
        }
        let parsed: Args = serde_json::from_str(args)?;
        Ok(Box::new(ToggleChannelMute {
            channel_id: parsed.channel_id,
        }))
    });

    dispatcher.register("toggle_channel_solo", |args| {
        #[derive(Deserialize)]
        struct Args {
            channel_id: ChannelId,
        }
        let parsed: Args = serde_json::from_str(args)?;
        Ok(Box::new(ToggleChannelSolo {
            channel_id: parsed.channel_id,
        }))
    });

    // Playlist commands
    dispatcher.register("add_clip", |args| {
        #[derive(Deserialize)]
        struct Args {
            track_id: TrackId,
            pattern_id: PatternId,
            start_beat: BeatTime,
            duration_beats: BeatTime,
        }
        let parsed: Args = serde_json::from_str(args)?;
        Ok(Box::new(AddClip {
            track_id: parsed.track_id,
            pattern_id: parsed.pattern_id,
            start_beat: parsed.start_beat,
            duration_beats: parsed.duration_beats,
        }))
    });

    dispatcher.register("set_bpm", |args| {
        #[derive(Deserialize)]
        struct Args {
            bpm: f64,
        }
        let parsed: Args = serde_json::from_str(args)?;
        Ok(Box::new(SetBpm { bpm: parsed.bpm }))
    });

    dispatcher.register("toggle_playback_mode", |_args| {
        Ok(Box::new(TogglePlaybackMode))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use daw_core::Project;

    #[test]
    fn test_create_pattern_command() {
        let mut project = Project::default();
        let cmd = CreatePattern {
            name: "Test".to_string(),
            color: "#ff0000".to_string(),
        };
        assert!(cmd.execute(&mut project).unwrap());
        assert_eq!(project.patterns.len(), 2); // 1 default + 1 new
    }

    #[test]
    fn test_toggle_step_command() {
        let mut project = Project::default();
        let pattern_id = project.current_pattern_id;

        let cmd = ToggleStep {
            pattern_id,
            channel_id: 0,
            step: 0,
        };
        assert!(cmd.execute(&mut project).unwrap());

        let pattern = project.get_pattern(pattern_id).unwrap();
        assert!(pattern.is_step_active(0, 0));
    }

    #[test]
    fn test_set_bpm_command() {
        let mut project = Project::default();
        let cmd = SetBpm { bpm: 140.0 };
        assert!(cmd.execute(&mut project).unwrap());
        assert_eq!(project.bpm, 140.0);
    }

    #[test]
    fn test_toggle_channel_mute() {
        let mut project = Project::default();
        let cmd = ToggleChannelMute { channel_id: 0 };
        assert!(!project.channels[0].muted);
        assert!(cmd.execute(&mut project).unwrap());
        assert!(project.channels[0].muted);
    }
}
