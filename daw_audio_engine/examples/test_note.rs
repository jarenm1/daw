use daw_audio_engine::midi_event::MidiEvent;
use daw_audio_engine::{AudioEngine, SineSynth};
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== Simple Note Test ===\n");

    let mut engine = AudioEngine::new()?;
    let sample_rate = engine
        .current_device()
        .map(|d| d.sample_rate)
        .unwrap_or(48000);

    // Create synth
    let mut synth = SineSynth::new(4, sample_rate);
    synth.set_gain(0.8);
    let mut synth = synth;

    // Manual audio processing test
    println!("Generating 1 second of C4 note...\n");

    // Trigger note
    let note_on = MidiEvent::note_on(60, 100, 0);
    synth.process_event(&note_on);

    // Render 1 second of audio at 48kHz
    let samples = (sample_rate as usize) * 2; // 1 second, stereo
    let mut buffer = vec![0.0f32; samples];

    // Process in chunks like the audio engine does
    let chunk_size = 512;
    for chunk in buffer.chunks_mut(chunk_size) {
        synth.render(chunk, 2);
    }

    // Check output
    let max_sample = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    println!("Max sample value: {:.4}", max_sample);

    if max_sample > 0.01 {
        println!("✓ Audio generated successfully!");
        println!("  Peak amplitude: {:.1}%", max_sample * 100.0);

        // Count non-zero samples
        let non_zero = buffer.iter().filter(|&&s| s.abs() > 0.001).count();
        println!("  Active samples: {} / {}", non_zero, buffer.len());
    } else {
        println!("✗ No audio output detected");
    }

    engine.start()?;

    // Now play it through the audio engine
    println!("\nPlaying through audio engine for 3 seconds...");

    let processor = TestProcessor::new(synth);
    engine.set_processor(Box::new(processor));

    std::thread::sleep(Duration::from_secs(3));

    engine.stop()?;
    println!("\nDone!");

    Ok(())
}

use daw_audio_engine::processor::{AudioProcessor, ProcessorConfig};
use std::f32::consts::PI;

struct TestProcessor {
    synth: SineSynth,
    started: Instant,
}

impl TestProcessor {
    fn new(synth: SineSynth) -> Self {
        Self {
            synth,
            started: Instant::now(),
        }
    }
}

impl AudioProcessor for TestProcessor {
    fn process(&mut self, _input: &[f32], output: &mut [f32]) {
        // Stop after 2 seconds
        if self.started.elapsed() > Duration::from_secs(2) {
            output.fill(0.0);
            return;
        }

        self.synth.render(output, 2);
    }

    fn configure(&mut self, _config: &ProcessorConfig) {}
    fn name(&self) -> &str {
        "Test"
    }
}
