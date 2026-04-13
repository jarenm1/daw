use crate::midi_event::pitch_to_freq;
use std::f32::consts::PI;

use super::Envelope;

/// A simple sine-wave voice for polyphonic synths.
pub struct Voice {
    active: bool,
    pitch: u8,
    frequency: f32,
    velocity: f32,
    phase: f32,
    envelope: Envelope,
}

impl Voice {
    pub fn new() -> Self {
        Self::with_sample_rate(48_000)
    }

    pub fn with_sample_rate(sample_rate: u32) -> Self {
        Self {
            active: false,
            pitch: 0,
            frequency: 0.0,
            velocity: 0.0,
            phase: 0.0,
            envelope: Envelope::with_sample_rate(sample_rate as f32),
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.envelope.set_sample_rate(sample_rate as f32);
    }

    pub fn set_adsr(&mut self, attack: f32, decay: f32, sustain: f32, release: f32) {
        self.envelope.set_adsr(attack, decay, sustain, release);
    }

    pub fn trigger(&mut self, pitch: u8, velocity: u8) {
        self.pitch = pitch;
        self.frequency = pitch_to_freq(pitch);
        self.velocity = velocity as f32 / 127.0;
        self.phase = 0.0;
        self.active = true;
        self.envelope.trigger();
    }

    pub fn release(&mut self) {
        self.envelope.release();
    }

    pub fn reset(&mut self) {
        self.active = false;
        self.pitch = 0;
        self.frequency = 0.0;
        self.velocity = 0.0;
        self.phase = 0.0;
        self.envelope.reset();
    }

    pub fn pitch(&self) -> u8 {
        self.pitch
    }

    pub fn is_active(&self) -> bool {
        self.active && self.envelope.is_active()
    }

    pub fn render_sine(&mut self, output: &mut [f32], channels: usize, sample_rate: u32) {
        if !self.is_active() {
            return;
        }

        let phase_inc = self.frequency / sample_rate as f32;
        let frames = output.len() / channels;

        for frame in 0..frames {
            let env_value = self.envelope.next_sample();
            let sample = (self.phase * 2.0 * PI).sin() * self.velocity * env_value;

            for channel in 0..channels {
                output[frame * channels + channel] += sample;
            }

            self.phase += phase_inc;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
        }

        if !self.envelope.is_active() {
            self.active = false;
        }
    }
}
