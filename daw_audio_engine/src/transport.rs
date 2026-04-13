use parking_lot::Mutex;
use std::sync::Arc;

/// Playback state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportState {
    Stopped,
    Playing,
    Paused,
    Recording,
}

/// Time signature (e.g., 4/4, 3/4)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeSignature {
    pub numerator: u8,   // Beats per bar (top number)
    pub denominator: u8, // Beat value (bottom number: 4=quarter, 8=eighth)
}

impl TimeSignature {
    pub const fn new(numerator: u8, denominator: u8) -> Self {
        Self {
            numerator,
            denominator,
        }
    }

    /// Common 4/4 time
    pub const FOUR_FOUR: Self = Self::new(4, 4);

    /// 3/4 waltz time
    pub const THREE_FOUR: Self = Self::new(3, 4);
}

impl Default for TimeSignature {
    fn default() -> Self {
        Self::FOUR_FOUR
    }
}

/// Transport handles all timing and playback state
pub struct Transport {
    state: TransportState,

    // Time position (all in seconds for simplicity, convert to bars/beats as needed)
    position: f64,

    // Timing
    bpm: f64,
    sample_rate: u32,
    time_signature: TimeSignature,

    // Loop settings
    loop_enabled: bool,
    loop_start: f64,
    loop_end: f64,

    // For sync with audio callback
    samples_processed: u64,

    // Shared state for UI monitoring
    shared: Arc<Mutex<TransportShared>>,
}

#[derive(Debug, Clone)]
struct TransportShared {
    position: f64,
    state: TransportState,
    bpm: f64,
}

impl Transport {
    pub fn new(sample_rate: u32) -> Self {
        let shared = Arc::new(Mutex::new(TransportShared {
            position: 0.0,
            state: TransportState::Stopped,
            bpm: 120.0,
        }));

        Self {
            state: TransportState::Stopped,
            position: 0.0,
            bpm: 120.0,
            sample_rate,
            time_signature: TimeSignature::default(),
            loop_enabled: false,
            loop_start: 0.0,
            loop_end: 4.0, // 4 seconds default loop
            samples_processed: 0,
            shared,
        }
    }

    /// Start playback
    pub fn play(&mut self) {
        if self.state == TransportState::Stopped {
            self.position = 0.0;
        }
        self.state = TransportState::Playing;
        self.update_shared();
    }

    /// Stop playback
    pub fn stop(&mut self) {
        self.state = TransportState::Stopped;
        self.position = 0.0;
        self.update_shared();
    }

    /// Pause playback (maintains position)
    pub fn pause(&mut self) {
        if self.state == TransportState::Playing {
            self.state = TransportState::Paused;
            self.update_shared();
        }
    }

    /// Toggle play/pause
    pub fn toggle_playback(&mut self) {
        match self.state {
            TransportState::Stopped => self.play(),
            TransportState::Playing => self.pause(),
            TransportState::Paused => self.play(),
            TransportState::Recording => self.stop(),
        }
    }

    /// Seek to position in seconds
    pub fn seek_to(&mut self, seconds: f64) {
        self.position = seconds.max(0.0);
        if self.loop_enabled && self.position >= self.loop_end {
            self.position = self.loop_start;
        }
        self.update_shared();
    }

    /// Seek to position in bars (measures)
    pub fn seek_to_bars(&mut self, bars: f64) {
        let seconds = self.bars_to_seconds(bars);
        self.seek_to(seconds);
    }

    /// Seek to position in beats
    pub fn seek_to_beats(&mut self, beats: f64) {
        let seconds = self.beats_to_seconds(beats);
        self.seek_to(seconds);
    }

    /// Get current state
    pub fn state(&self) -> TransportState {
        self.state
    }

    /// Is currently playing?
    pub fn is_playing(&self) -> bool {
        self.state == TransportState::Playing
    }

    /// Get position in seconds
    pub fn position(&self) -> f64 {
        self.position
    }

    /// Get position in bars (measures)
    pub fn position_bars(&self) -> f64 {
        self.seconds_to_bars(self.position)
    }

    /// Get position in beats
    pub fn position_beats(&self) -> f64 {
        self.seconds_to_beats(self.position)
    }

    /// Get position as formatted string (bars.beats.ticks)
    pub fn position_formatted(&self) -> String {
        let total_beats = self.position_beats();
        let bars = total_beats as u32 / self.time_signature.numerator as u32;
        let beats = (total_beats as u32 % self.time_signature.numerator as u32) + 1;
        let ticks = ((total_beats.fract()) * 960.0) as u32; // 960 ticks per beat

        format!("{:03}.{:02}.{:03}", bars, beats, ticks)
    }

    /// Set BPM
    pub fn set_bpm(&mut self, bpm: f64) {
        self.bpm = bpm.clamp(1.0, 999.0);
        self.update_shared();
    }

    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    /// Set time signature
    pub fn set_time_signature(&mut self, sig: TimeSignature) {
        self.time_signature = sig;
    }

    pub fn time_signature(&self) -> TimeSignature {
        self.time_signature
    }

    /// Enable/disable loop
    pub fn set_loop(&mut self, enabled: bool) {
        self.loop_enabled = enabled;
    }

    pub fn is_looping(&self) -> bool {
        self.loop_enabled
    }

    /// Set loop range in seconds
    pub fn set_loop_range(&mut self, start: f64, end: f64) {
        self.loop_start = start.min(end);
        self.loop_end = end.max(start);
    }

    pub fn loop_range(&self) -> (f64, f64) {
        (self.loop_start, self.loop_end)
    }

    /// Advance position by sample count (called from audio thread)
    pub fn advance(&mut self, samples: usize) {
        if self.state != TransportState::Playing {
            return;
        }

        let seconds = samples as f64 / self.sample_rate as f64;
        self.position += seconds;
        self.samples_processed += samples as u64;

        // Handle looping
        if self.loop_enabled && self.position >= self.loop_end {
            self.position = self.loop_start;
        }

        // Periodically update shared state (every ~100ms)
        if self.samples_processed % (self.sample_rate as u64 / 10) == 0 {
            self.update_shared();
        }
    }

    /// Convert seconds to bars
    pub fn seconds_to_bars(&self, seconds: f64) -> f64 {
        self.seconds_to_beats(seconds) / self.time_signature.numerator as f64
    }

    /// Convert bars to seconds
    pub fn bars_to_seconds(&self, bars: f64) -> f64 {
        self.beats_to_seconds(bars * self.time_signature.numerator as f64)
    }

    /// Convert seconds to beats
    pub fn seconds_to_beats(&self, seconds: f64) -> f64 {
        seconds * self.bpm / 60.0
    }

    /// Convert beats to seconds
    pub fn beats_to_seconds(&self, beats: f64) -> f64 {
        beats * 60.0 / self.bpm
    }

    /// Get a handle for monitoring from UI thread
    pub fn get_monitor(&self) -> TransportMonitor {
        TransportMonitor {
            shared: self.shared.clone(),
        }
    }

    fn update_shared(&self) {
        let mut shared = self.shared.lock();
        shared.position = self.position;
        shared.state = self.state;
        shared.bpm = self.bpm;
    }
}

/// Monitor handle for UI thread (non-blocking reads)
pub struct TransportMonitor {
    shared: Arc<Mutex<TransportShared>>,
}

impl TransportMonitor {
    pub fn position(&self) -> f64 {
        self.shared.lock().position
    }

    pub fn state(&self) -> TransportState {
        self.shared.lock().state
    }

    pub fn bpm(&self) -> f64 {
        self.shared.lock().bpm
    }
}

/// Builder for transport
pub struct TransportBuilder {
    sample_rate: u32,
    bpm: f64,
    time_signature: TimeSignature,
}

impl TransportBuilder {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            bpm: 120.0,
            time_signature: TimeSignature::default(),
        }
    }

    pub fn bpm(mut self, bpm: f64) -> Self {
        self.bpm = bpm;
        self
    }

    pub fn time_signature(mut self, num: u8, den: u8) -> Self {
        self.time_signature = TimeSignature::new(num, den);
        self
    }

    pub fn build(self) -> Transport {
        let mut transport = Transport::new(self.sample_rate);
        transport.set_bpm(self.bpm);
        transport.set_time_signature(self.time_signature);
        transport
    }
}
