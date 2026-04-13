use daw_audio_engine::midi_event::MidiNote;
use daw_audio_engine::{AudioEngine, MidiClip, SineSynth, Timeline, TransportState};
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== Clean Chord Demo ===\n");

    let mut engine = AudioEngine::new()?;
    let sample_rate = engine
        .current_device()
        .map(|d| d.sample_rate)
        .unwrap_or(48000);

    // Only 3 voices - exactly one chord
    let mut synth = SineSynth::new(3, sample_rate);
    synth.set_gain(0.8);
    let synth = Arc::new(Mutex::new(synth));
    let synth_clone = synth.clone();

    let processor = CleanSynthProcessor::new(synth);
    engine.set_processor(Box::new(processor));

    engine.start()?;
    println!(
        "✓ Audio at {} Hz (3 voices max = one clean chord)\n",
        sample_rate
    );

    let mut timeline = Timeline::with_bpm("Chords", sample_rate, 120.0);
    let clip = create_clean_chords();

    let mut track = daw_audio_engine::MidiTrack::new(
        "Synth",
        std::sync::Arc::new(daw_audio_engine::SimpleSynth::new(4)),
    );
    track.add_clip(clip);
    timeline.sequencer.add_track(track);

    timeline.sequencer.transport_mut().set_loop(true);
    timeline.sequencer.transport_mut().set_loop_range(0.0, 8.0);

    println!("Chord Progression (each chord is distinct):");
    println!("  0-1.5s: C Major");
    println!("  2-3.5s: F Major");
    println!("  4-5.5s: G Major");
    println!("  6-7.5s: C Major\n");

    println!("Controls: p=play/pause, s=stop, q=quit");
    println!("Starting...\n");
    std::thread::sleep(Duration::from_millis(500));

    timeline.play();

    let start = Instant::now();
    let mut last_update = Instant::now();

    loop {
        let transport = timeline.sequencer.transport();
        let pos = transport.position();
        let state = transport.state();

        if last_update.elapsed() >= Duration::from_millis(100) {
            let chord = get_chord(pos);
            let status = match state {
                TransportState::Playing => "▶",
                _ => "■",
            };

            let voices = synth_clone.lock().unwrap().active_voice_count();
            print!(
                "\r{} {:.1}s | {} | {} voices    ",
                status, pos, chord, voices
            );
            io::stdout().flush()?;
            last_update = Instant::now();
        }

        if state == TransportState::Playing {
            let events = timeline.process(256);

            for event in events {
                synth_clone.lock().unwrap().process_event(&event);
            }
        }

        let mut buf = [0u8; 1];
        if io::stdin().read(&mut buf).is_ok() {
            match buf[0] as char {
                'p' => timeline.toggle(),
                's' => {
                    timeline.stop();
                    timeline.seek(0.0);
                }
                'q' => break,
                _ => {}
            }
        }

        if start.elapsed() > Duration::from_secs(30) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    timeline.stop();
    engine.stop()?;
    println!("\n\nDone!");

    Ok(())
}

fn create_clean_chords() -> MidiClip {
    let mut clip = MidiClip::new("Chords", 0.0, 8.0);

    // Clear 0.5s gap between chords for distinct sound
    // C Major at 0s
    clip.add_note(MidiNote::new(60, 100, 0.0, 1.5));
    clip.add_note(MidiNote::new(64, 100, 0.0, 1.5));
    clip.add_note(MidiNote::new(67, 100, 0.0, 1.5));

    // F Major at 2s
    clip.add_note(MidiNote::new(65, 100, 2.0, 1.5));
    clip.add_note(MidiNote::new(69, 100, 2.0, 1.5));
    clip.add_note(MidiNote::new(72, 100, 2.0, 1.5));

    // G Major at 4s
    clip.add_note(MidiNote::new(67, 100, 4.0, 1.5));
    clip.add_note(MidiNote::new(71, 100, 4.0, 1.5));
    clip.add_note(MidiNote::new(74, 100, 4.0, 1.5));

    // C Major at 6s
    clip.add_note(MidiNote::new(60, 100, 6.0, 1.5));
    clip.add_note(MidiNote::new(64, 100, 6.0, 1.5));
    clip.add_note(MidiNote::new(67, 100, 6.0, 1.5));

    clip
}

fn get_chord(time: f64) -> &'static str {
    let t = time % 8.0;
    match t {
        0.0..=1.5 => "C Major",
        2.0..=3.5 => "F Major",
        4.0..=5.5 => "G Major",
        _ => "C Major",
    }
}

use daw_audio_engine::processor::{AudioProcessor, ProcessorConfig};

struct CleanSynthProcessor {
    synth: Arc<Mutex<SineSynth>>,
    config: Option<ProcessorConfig>,
}

impl CleanSynthProcessor {
    fn new(synth: Arc<Mutex<SineSynth>>) -> Self {
        Self {
            synth,
            config: None,
        }
    }
}

impl AudioProcessor for CleanSynthProcessor {
    fn process(&mut self, _input: &[f32], output: &mut [f32]) {
        let channels = self.config.as_ref().map(|c| c.output_channels).unwrap_or(2);
        if let Ok(mut synth) = self.synth.try_lock() {
            synth.render(output, channels);
        } else {
            output.fill(0.0);
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
        "CleanSynth"
    }
}
