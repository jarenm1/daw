use crate::instrument::VirtualInstrument;
use crate::midi_event::{MidiEvent, MidiEventType};

use super::Voice;

/// A direct polyphonic sine synth with explicit rendering controls.
pub struct SineSynth {
    voices: Vec<Voice>,
    gain: f32,
    sample_rate: u32,
}

impl SineSynth {
    pub fn new(max_voices: usize, sample_rate: u32) -> Self {
        let mut voices = Vec::with_capacity(max_voices);
        for _ in 0..max_voices {
            voices.push(Voice::with_sample_rate(sample_rate));
        }

        Self {
            voices,
            gain: 0.3,
            sample_rate,
        }
    }

    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain.clamp(0.0, 1.0);
    }

    pub fn get_gain(&self) -> f32 {
        self.gain
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate;
        for voice in &mut self.voices {
            voice.set_sample_rate(sample_rate);
        }
    }

    pub fn set_adsr(&mut self, attack: f32, decay: f32, sustain: f32, release: f32) {
        for voice in &mut self.voices {
            voice.set_adsr(attack, decay, sustain, release);
        }
    }

    pub fn process_event(&mut self, event: &MidiEvent) {
        match event.event_type {
            MidiEventType::NoteOn { pitch, velocity } => {
                if velocity > 0 {
                    if let Some(voice) = self.find_free_voice(pitch) {
                        voice.trigger(pitch, velocity);
                    }
                } else {
                    self.release_voice(pitch);
                }
            }
            MidiEventType::NoteOff { pitch } => self.release_voice(pitch),
            _ => {}
        }
    }

    pub fn render(&mut self, output: &mut [f32], channels: usize) {
        output.fill(0.0);

        for voice in &mut self.voices {
            if voice.is_active() {
                voice.render_sine(output, channels, self.sample_rate);
            }
        }

        for sample in output.iter_mut() {
            *sample *= self.gain;
        }
    }

    pub fn reset(&mut self) {
        for voice in &mut self.voices {
            voice.reset();
        }
    }

    pub fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|voice| voice.is_active()).count()
    }

    pub fn kill_all_voices(&mut self) {
        self.reset();
    }

    fn find_free_voice(&mut self, for_pitch: u8) -> Option<&mut Voice> {
        for index in 0..self.voices.len() {
            if !self.voices[index].is_active() {
                return Some(&mut self.voices[index]);
            }
        }

        for index in 0..self.voices.len() {
            if self.voices[index].pitch() == for_pitch {
                self.voices[index].reset();
                return Some(&mut self.voices[index]);
            }
        }

        if !self.voices.is_empty() {
            self.voices[0].reset();
            return Some(&mut self.voices[0]);
        }

        None
    }

    fn release_voice(&mut self, pitch: u8) {
        for voice in &mut self.voices {
            if voice.pitch() == pitch && voice.is_active() {
                voice.release();
                break;
            }
        }
    }
}

impl VirtualInstrument for SineSynth {
    fn handle_event(&mut self, event: &MidiEvent) {
        SineSynth::process_event(self, event);
    }

    fn render(&mut self, output: &mut [f32], channels: usize, sample_rate: u32) {
        if sample_rate != self.sample_rate {
            self.set_sample_rate(sample_rate);
        }
        SineSynth::render(self, output, channels);
    }

    fn name(&self) -> &str {
        "SineSynth"
    }

    fn reset(&mut self) {
        SineSynth::reset(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sine_synth() {
        let mut synth = SineSynth::new(8, 48_000);
        let mut output = vec![0.0_f32; 1024];

        synth.process_event(&MidiEvent::note_on(60, 100, 0));
        synth.render(&mut output, 2);

        let max_sample = output
            .iter()
            .map(|sample| sample.abs())
            .fold(0.0_f32, f32::max);
        assert!(max_sample > 0.001, "Synth should produce audio");
    }
}
