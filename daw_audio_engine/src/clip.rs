use crate::buffer::AudioBuffer;
use crate::error::{AudioError, AudioResult};
use crate::file_io::{load_audio_file, save_wav_file, AudioFileInfo};
use parking_lot::Mutex;
use std::path::Path;
use std::sync::Arc;

/// Represents an audio clip - loaded audio data that can be played
#[derive(Clone)]
pub struct AudioClip {
    data: Arc<Mutex<ClipData>>,
    info: AudioFileInfo,
}

struct ClipData {
    buffer: AudioBuffer,
    sample_rate: u32,
    is_resampled: bool,
}

impl AudioClip {
    /// Load an audio file into a clip
    pub fn from_file<P: AsRef<Path>>(path: P) -> AudioResult<Self> {
        let (buffer, info) = load_audio_file(path)?;

        Ok(Self {
            data: Arc::new(Mutex::new(ClipData {
                buffer,
                sample_rate: info.sample_rate,
                is_resampled: false,
            })),
            info,
        })
    }

    /// Create clip from raw buffer
    pub fn from_buffer(buffer: AudioBuffer, sample_rate: u32) -> Self {
        let info = AudioFileInfo {
            sample_rate,
            channels: buffer.channels(),
            sample_count: buffer.frames(),
            duration_secs: buffer.frames() as f64 / sample_rate as f64,
            format: crate::file_io::AudioFormat::Wav,
            bit_depth: 32,
        };

        Self {
            data: Arc::new(Mutex::new(ClipData {
                buffer,
                sample_rate,
                is_resampled: false,
            })),
            info,
        }
    }

    /// Get clip info
    pub fn info(&self) -> &AudioFileInfo {
        &self.info
    }

    /// Get duration in seconds
    pub fn duration(&self) -> f64 {
        self.info.duration_secs
    }

    /// Get number of channels
    pub fn channels(&self) -> usize {
        self.info.channels
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> u32 {
        self.info.sample_rate
    }

    /// Access the raw buffer
    pub fn with_buffer<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&AudioBuffer) -> R,
    {
        let data = self.data.lock();
        f(&data.buffer)
    }

    /// Resample clip to target sample rate
    pub fn resample_to(&self, target_rate: u32) {
        let mut data = self.data.lock();

        if data.sample_rate != target_rate {
            use crate::file_io::resample;
            data.buffer = resample(&data.buffer, data.sample_rate, target_rate);
            data.sample_rate = target_rate;
            data.is_resampled = true;
        }
    }

    /// Save clip to WAV file
    pub fn save_to_wav<P: AsRef<Path>>(&self, path: P) -> AudioResult<()> {
        let data = self.data.lock();
        save_wav_file(path, &data.buffer, data.sample_rate)
    }

    /// Get sample at specific frame and channel
    pub fn sample(&self, channel: usize, frame: usize) -> f32 {
        let data = self.data.lock();
        if frame < data.buffer.frames() && channel < data.buffer.channels() {
            data.buffer.sample(channel, frame)
        } else {
            0.0
        }
    }

    /// Read interleaved samples into output buffer
    pub fn read_samples(&self, offset: usize, output: &mut [f32]) -> usize {
        let data = self.data.lock();
        let buffer = &data.buffer;
        let available = buffer.frames().saturating_sub(offset);
        let to_copy = (output.len() / buffer.channels()).min(available);

        if to_copy == 0 {
            return 0;
        }

        let src_start = offset * buffer.channels();
        let src_end = src_start + to_copy * buffer.channels();
        let src = &buffer.as_interleaved()[src_start..src_end];

        output[..src.len()].copy_from_slice(src);

        to_copy
    }
}

/// Playback state for a clip
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

/// A voice that can play audio clips
pub struct AudioPlayer {
    state: PlaybackState,
    position: usize, // Current frame position
    clip: Option<AudioClip>,
    gain: f32, // Volume (0.0 - 1.0+)
    loop_playback: bool,
    channels: usize, // Output channels
}

impl AudioPlayer {
    pub fn new(channels: usize) -> Self {
        Self {
            state: PlaybackState::Stopped,
            position: 0,
            clip: None,
            gain: 1.0,
            loop_playback: false,
            channels,
        }
    }

    /// Load a clip to play
    pub fn load_clip(&mut self, clip: AudioClip) {
        self.clip = Some(clip);
        self.position = 0;
        self.state = PlaybackState::Stopped;
    }

    /// Start playback
    pub fn play(&mut self) {
        if self.clip.is_some() {
            self.state = PlaybackState::Playing;
        }
    }

    /// Stop playback
    pub fn stop(&mut self) {
        self.state = PlaybackState::Stopped;
        self.position = 0;
    }

    /// Pause playback
    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            self.state = PlaybackState::Paused;
        }
    }

    /// Get current state
    pub fn state(&self) -> PlaybackState {
        self.state
    }

    /// Is currently playing?
    pub fn is_playing(&self) -> bool {
        self.state == PlaybackState::Playing
    }

    /// Set volume (0.0 = silent, 1.0 = normal, >1.0 = boosted)
    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain.clamp(0.0, 10.0);
    }

    pub fn gain(&self) -> f32 {
        self.gain
    }

    /// Enable/disable looping
    pub fn set_loop(&mut self, loop_enabled: bool) {
        self.loop_playback = loop_enabled;
    }

    pub fn is_looping(&self) -> bool {
        self.loop_playback
    }

    /// Get current position in frames
    pub fn position(&self) -> usize {
        self.position
    }

    /// Get position as seconds
    pub fn position_secs(&self) -> f64 {
        if let Some(ref clip) = self.clip {
            self.position as f64 / clip.sample_rate() as f64
        } else {
            0.0
        }
    }

    /// Seek to position in frames
    pub fn seek_to(&mut self, frame: usize) {
        if let Some(ref clip) = self.clip {
            self.position = frame.min(clip.with_buffer(|b| b.frames()));
        }
    }

    /// Seek to position in seconds
    pub fn seek_to_secs(&mut self, secs: f64) {
        if let Some(ref clip) = self.clip {
            let frame = (secs * clip.sample_rate() as f64) as usize;
            self.seek_to(frame);
        }
    }

    /// Process audio - fill output buffer with clip samples
    pub fn process(&mut self, output: &mut [f32]) {
        if self.state != PlaybackState::Playing || self.clip.is_none() {
            output.fill(0.0);
            return;
        }

        let clip = self.clip.as_ref().unwrap();
        let channels = self.channels;
        let frames = output.len() / channels;
        let mut written = 0;

        while written < frames {
            let remaining = frames - written;
            let offset = self.position;

            // Get temporary buffer for reading
            let mut temp = vec![0.0f32; remaining * channels];
            let read = clip.read_samples(offset, &mut temp);

            if read == 0 {
                // End of clip
                if self.loop_playback {
                    self.position = 0;
                    continue;
                } else {
                    // Fill rest with silence
                    output[written * channels..].fill(0.0);
                    self.state = PlaybackState::Stopped;
                    break;
                }
            }

            // Apply gain and write to output (handle channel conversion)
            let clip_channels = clip.channels();
            for frame in 0..read {
                for ch in 0..channels {
                    let src_ch = ch.min(clip_channels - 1);
                    let sample = temp[frame * clip_channels + src_ch] * self.gain;
                    output[(written + frame) * channels + ch] = sample;
                }
            }

            written += read;
            self.position += read;
        }
    }

    /// Get clip reference if loaded
    pub fn clip(&self) -> Option<&AudioClip> {
        self.clip.as_ref()
    }

    /// Unload clip
    pub fn unload(&mut self) {
        self.stop();
        self.clip = None;
    }
}
