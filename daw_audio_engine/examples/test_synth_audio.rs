use daw_audio_engine::midi_event::MidiEvent;
use daw_audio_engine::{AudioEngine, SineSynth};
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== Audio Output Test ===\n");

    let mut engine = AudioEngine::new()?;
    let sample_rate = engine
        .current_device()
        .map(|d| d.sample_rate)
        .unwrap_or(48000);

    // Create synth with higher gain
    let mut synth = SineSynth::new(8, sample_rate);
    synth.set_gain(0.8); // 80% volume (was 30%)

    // Create processor
    let processor = TestSynthProcessor::new(synth);
    engine.set_processor(Box::new(processor));

    engine.start()?;
    println!("✓ Audio engine started at {} Hz", sample_rate);
    println!("✓ Synth gain: 80%\n");

    // Trigger a note manually
    println!("Playing C4 (MIDI 60) for 3 seconds...");
    let note_on = MidiEvent::note_on(60, 127, 0); // Max velocity

    // We need to get the synth to the processor...
    // Let me create a different approach - direct test

    std::thread::sleep(Duration::from_secs(3));

    println!("\nIf you heard a tone, the synth works!");
    println!("If not, check system volume.");

    engine.stop()?;
    Ok(())
}

use daw_audio_engine::processor::{AudioProcessor, ProcessorConfig};

struct TestSynthProcessor {
    synth: SineSynth,
    config: Option<ProcessorConfig>,
    start_time: Instant,
}

impl TestSynthProcessor {
    fn new(mut synth: SineSynth) -> Self {
        // Trigger a note immediately
        let note = daw_audio_engine::midi_event::MidiEvent::note_on(60, 127, 0);
        synth.process_event(&note);

        Self {
            synth,
            config: None,
            start_time: Instant::now(),
        }
    }
}

impl AudioProcessor for TestSynthProcessor {
    fn process(&mut self, _input: &[f32], output: &mut [f32]) {
        let channels = self.config.as_ref().map(|c| c.output_channels).unwrap_or(2);

        // Release note after 2 seconds
        if self.start_time.elapsed() > Duration::from_secs(2) {
            let off = daw_audio_engine::midi_event::MidiEvent::note_off(60, 0);
            self.synth.process_event(&off);
        }

        self.synth.render(output, channels);

        // Debug: print max sample value
        let max = output.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        if max > 0.001 {
            eprintln!("Audio output: max={:.4}", max);
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
        "TestSynth"
    }
}
