use crate::instrument::VirtualInstrument;
use crate::midi_event::MidiEvent;

use super::SineSynth;

/// A convenience synth wrapper that defaults to the engine sample rate.
pub struct SimpleSynth {
    inner: SineSynth,
}

impl SimpleSynth {
    pub fn new(max_voices: usize) -> Self {
        Self {
            inner: SineSynth::new(max_voices, 48_000),
        }
    }

    pub fn set_gain(&mut self, gain: f32) {
        self.inner.set_gain(gain);
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.inner.set_sample_rate(sample_rate);
    }

    pub fn set_adsr(&mut self, attack: f32, decay: f32, sustain: f32, release: f32) {
        self.inner.set_adsr(attack, decay, sustain, release);
    }

    pub fn active_voice_count(&self) -> usize {
        self.inner.active_voice_count()
    }
}

impl VirtualInstrument for SimpleSynth {
    fn handle_event(&mut self, event: &MidiEvent) {
        self.inner.handle_event(event);
    }

    fn render(&mut self, output: &mut [f32], channels: usize, sample_rate: u32) {
        self.inner.set_sample_rate(sample_rate);
        self.inner.render(output, channels);
    }

    fn name(&self) -> &str {
        "SimpleSynth"
    }

    fn reset(&mut self) {
        self.inner.reset();
    }
}
