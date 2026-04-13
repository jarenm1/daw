//! Channel data model
//!
//! A channel represents an instrument/sampler in the channel rack.

use serde::{Deserialize, Serialize};

use crate::Id;

/// Unique identifier for a channel
pub type ChannelId = Id;

/// A channel represents an instrument/sampler in the channel rack
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Channel {
    pub id: ChannelId,
    pub name: String,
    /// Color for UI representation
    pub color: String,
    /// Channel type determines how it generates sound
    pub channel_type: ChannelType,
    /// Volume (0.0 - 1.0)
    pub volume: f32,
    /// Pan (-1.0 left to 1.0 right)
    pub pan: f32,
    /// Mute state
    pub muted: bool,
    /// Solo state
    pub solo: bool,
    /// Output to mixer track
    pub mixer_track: usize,
}

/// Types of channels
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ChannelType {
    /// Sample-based instrument
    Sampler {
        /// Path to audio file
        sample_path: Option<String>,
        /// Root note for sample playback (if different from C4)
        root_note: Option<u8>,
        /// Loop sample
        loop_sample: bool,
    },
    /// Built-in synthesizer
    Synthesizer {
        /// Preset name
        preset: String,
        /// Oscillator type
        osc_type: OscillatorType,
    },
    /// Audio clip player
    AudioClip {
        /// Path to audio file
        file_path: String,
    },
    /// External plugin (VST, etc.)
    Plugin {
        /// Plugin name
        name: String,
        /// Plugin vendor
        vendor: Option<String>,
        /// Plugin type
        plugin_type: PluginType,
    },
}

/// Oscillator types for synthesizers
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum OscillatorType {
    Sine,
    Square,
    Sawtooth,
    Triangle,
    Noise,
    Custom(String),
}

/// Plugin types
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PluginType {
    Vst2,
    Vst3,
    Au, // Audio Unit (macOS)
    Clap,
    Lv2,
    Unknown,
}

impl Channel {
    /// Create a new channel
    pub fn new(
        id: ChannelId,
        name: impl Into<String>,
        color: impl Into<String>,
        channel_type: ChannelType,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            color: color.into(),
            channel_type,
            volume: 0.8,
            pan: 0.0,
            muted: false,
            solo: false,
            mixer_track: 0,
        }
    }

    /// Create a new synthesizer channel
    pub fn new_synth(id: ChannelId, name: impl Into<String>, color: impl Into<String>) -> Self {
        Self::new(
            id,
            name,
            color,
            ChannelType::Synthesizer {
                preset: "Default".to_string(),
                osc_type: OscillatorType::Sine,
            },
        )
    }

    /// Create a new sampler channel
    pub fn new_sampler(
        id: ChannelId,
        name: impl Into<String>,
        color: impl Into<String>,
        sample_path: Option<String>,
    ) -> Self {
        Self::new(
            id,
            name,
            color,
            ChannelType::Sampler {
                sample_path,
                root_note: None,
                loop_sample: false,
            },
        )
    }

    /// Set volume (clamped 0.0 - 1.0)
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    /// Set pan (clamped -1.0 - 1.0)
    pub fn set_pan(&mut self, pan: f32) {
        self.pan = pan.clamp(-1.0, 1.0);
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

    /// Check if channel is effectively muted (considering solo states of other channels)
    pub fn is_effective_mute(&self, any_solo_active: bool) -> bool {
        self.muted || (any_solo_active && !self.solo)
    }

    /// Get the output gain factor (volume with mute applied)
    pub fn effective_gain(&self, any_solo_active: bool) -> f32 {
        if self.is_effective_mute(any_solo_active) {
            0.0
        } else {
            self.volume
        }
    }
}

impl Default for Channel {
    fn default() -> Self {
        Self::new_synth(0, "Channel 1", "#3b82f6")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_new() {
        let channel = Channel::new_synth(1, "Bass", "#ff0000");
        assert_eq!(channel.id, 1);
        assert_eq!(channel.name, "Bass");
        assert!(!channel.muted);
        assert!(!channel.solo);
    }

    #[test]
    fn test_volume_clamping() {
        let mut channel = Channel::default();
        channel.set_volume(1.5);
        assert_eq!(channel.volume, 1.0);
        channel.set_volume(-0.5);
        assert_eq!(channel.volume, 0.0);
    }

    #[test]
    fn test_pan_clamping() {
        let mut channel = Channel::default();
        channel.set_pan(2.0);
        assert_eq!(channel.pan, 1.0);
        channel.set_pan(-2.0);
        assert_eq!(channel.pan, -1.0);
    }

    #[test]
    fn test_effective_mute() {
        let mut channel = Channel::default();
        assert!(!channel.is_effective_mute(false));
        assert!(!channel.is_effective_mute(true)); // soloed by default when no solos

        channel.toggle_mute();
        assert!(channel.is_effective_mute(false));

        channel.toggle_mute();
        channel.toggle_solo();
        assert!(!channel.is_effective_mute(true)); // this one is soloed
        assert!(channel.is_effective_mute(false)); // but muted when no solos active
    }
}
