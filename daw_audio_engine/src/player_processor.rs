use crate::clip::{AudioClip, AudioPlayer};
use crate::processor::{AudioProcessor, ProcessorConfig};
use crossbeam::channel::{bounded, Receiver, Sender};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Commands that can be sent to the player
#[derive(Debug, Clone, Copy)]
pub enum PlayerCommand {
    Play,
    Stop,
    Pause,
    SetGain(f32),
    SetLoop(bool),
    Seek(usize), // frame position
}

/// Processor that plays audio clips through the engine
/// Uses message passing to avoid lock contention with audio thread
pub struct ClipPlayerProcessor {
    player: AudioPlayer,
    command_rx: Receiver<PlayerCommand>,
    position: Arc<AtomicUsize>, // For monitoring from main thread
    config: Option<ProcessorConfig>,
}

/// Control handle for the player (can be used from any thread)
pub struct PlayerHandle {
    command_tx: Sender<PlayerCommand>,
    position: Arc<AtomicUsize>,
    clip_duration_frames: usize,
    sample_rate: u32,
    channels: usize,
}

impl PlayerHandle {
    pub fn play(&self) {
        let _ = self.command_tx.try_send(PlayerCommand::Play);
    }

    pub fn stop(&self) {
        let _ = self.command_tx.try_send(PlayerCommand::Stop);
    }

    pub fn pause(&self) {
        let _ = self.command_tx.try_send(PlayerCommand::Pause);
    }

    pub fn set_gain(&self, gain: f32) {
        let _ = self.command_tx.try_send(PlayerCommand::SetGain(gain));
    }

    pub fn set_loop(&self, loop_enabled: bool) {
        let _ = self
            .command_tx
            .try_send(PlayerCommand::SetLoop(loop_enabled));
    }

    pub fn seek_to_frame(&self, frame: usize) {
        let _ = self.command_tx.try_send(PlayerCommand::Seek(frame));
    }

    pub fn seek_to_secs(&self, secs: f64) {
        let frame = (secs * self.sample_rate as f64) as usize;
        self.seek_to_frame(frame.min(self.clip_duration_frames));
    }

    /// Get current position in frames (approximate, may be slightly behind)
    pub fn position(&self) -> usize {
        self.position.load(Ordering::Relaxed)
    }

    /// Get current position in seconds
    pub fn position_secs(&self) -> f64 {
        self.position() as f64 / self.sample_rate as f64
    }

    pub fn duration_secs(&self) -> f64 {
        self.clip_duration_frames as f64 / self.sample_rate as f64
    }

    pub fn channels(&self) -> usize {
        self.channels
    }
}

impl ClipPlayerProcessor {
    pub fn with_clip(clip: AudioClip, channels: usize) -> (Self, PlayerHandle) {
        let sample_rate = clip.sample_rate();
        let clip_duration = clip.with_buffer(|b| b.frames());

        let mut player = AudioPlayer::new(channels);
        player.load_clip(clip);

        let (command_tx, command_rx) = bounded(64); // 64 command buffer
        let position = Arc::new(AtomicUsize::new(0));

        let processor = Self {
            player,
            command_rx,
            position: position.clone(),
            config: None,
        };

        let handle = PlayerHandle {
            command_tx,
            position,
            clip_duration_frames: clip_duration,
            sample_rate,
            channels,
        };

        (processor, handle)
    }
}

impl AudioProcessor for ClipPlayerProcessor {
    fn process(&mut self, _input: &[f32], output: &mut [f32]) {
        // Process all pending commands first
        while let Ok(cmd) = self.command_rx.try_recv() {
            match cmd {
                PlayerCommand::Play => self.player.play(),
                PlayerCommand::Stop => self.player.stop(),
                PlayerCommand::Pause => self.player.pause(),
                PlayerCommand::SetGain(g) => self.player.set_gain(g),
                PlayerCommand::SetLoop(l) => self.player.set_loop(l),
                PlayerCommand::Seek(pos) => self.player.seek_to(pos),
            }
        }

        // Process audio
        self.player.process(output);

        // Update position for monitoring (only if playing to avoid unnecessary atomic ops)
        if self.player.is_playing() {
            self.position
                .store(self.player.position(), Ordering::Relaxed);
        }
    }

    fn configure(&mut self, config: &ProcessorConfig) {
        self.config = Some(ProcessorConfig {
            sample_rate: config.sample_rate,
            buffer_size: config.buffer_size,
            input_channels: config.input_channels,
            output_channels: config.output_channels,
        });
    }

    fn name(&self) -> &str {
        "ClipPlayer"
    }
}
