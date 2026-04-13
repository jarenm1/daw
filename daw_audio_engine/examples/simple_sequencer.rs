use daw_audio_engine::midi_event::MidiNote;
use daw_audio_engine::{AudioEngine, MidiClip, SineSynth, Timeline, TransportState};
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== Simple MIDI Sequencer ===\n");

    // Initialize audio engine
    let mut engine = AudioEngine::new()?;
    let sample_rate = engine
        .current_device()
        .map(|d| d.sample_rate)
        .unwrap_or(48000);

    // Create synth with proper volume
    let mut synth_instance = SineSynth::new(16, sample_rate);
    synth_instance.set_gain(0.8); // 80% volume
    let synth = Arc::new(Mutex::new(synth_instance));
    let synth_clone = synth.clone();

    // Create processor
    let processor = SimpleSynthProcessor::new(synth);
    engine.set_processor(Box::new(processor));

    engine.start()?;
    println!("✓ Audio engine at {} Hz\n", sample_rate);

    // Create timeline
    let mut timeline = Timeline::with_bpm("Chord Demo", sample_rate, 120.0);

    // Create chord progression clip
    let clip = create_chord_progression();

    // Add to track
    let mut track = daw_audio_engine::MidiTrack::new(
        "Synth",
        std::sync::Arc::new(daw_audio_engine::SimpleSynth::new(8)),
    );
    track.add_clip(clip);
    timeline.sequencer.add_track(track);

    // Enable looping
    timeline.sequencer.transport_mut().set_loop(true);
    timeline.sequencer.transport_mut().set_loop_range(0.0, 8.0);

    println!("Chord Progression:");
    println!("  0-2s: C Major (C-E-G)");
    println!("  2-4s: F Major (F-A-C)");
    println!("  4-6s: G Major (G-B-D)");
    println!("  6-8s: C Major (C-E-G)\n");

    println!("Controls: p=play/pause, s=stop, q=quit");
    println!("Starting in 1 second...\n");
    std::thread::sleep(Duration::from_secs(1));

    timeline.play();

    // Main loop
    let start = Instant::now();
    let mut last_update = Instant::now();

    loop {
        let transport = timeline.sequencer.transport();
        let pos = transport.position();
        let state = transport.state();

        // Update display
        if last_update.elapsed() >= Duration::from_millis(100) {
            let chord = get_chord_name(pos);
            let status = match state {
                TransportState::Playing => "▶ PLAY",
                _ => "■ STOP",
            };

            let voices = synth_clone.lock().unwrap().active_voice_count();
            print!(
                "\r[{}] {:.1}s | {} | {} voices        ",
                status, pos, chord, voices
            );
            io::stdout().flush()?;
            last_update = Instant::now();
        }

        // Process MIDI events
        if state == TransportState::Playing {
            let events = timeline.process(256);
            for event in events {
                synth_clone.lock().unwrap().process_event(&event);
            }
        }

        // Check input
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

        if start.elapsed() > Duration::from_secs(60) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    timeline.stop();
    engine.stop()?;
    println!("\n\nDone!");

    Ok(())
}

fn create_chord_progression() -> MidiClip {
    let mut clip = MidiClip::new("Chords", 0.0, 8.0);

    // Each chord plays for 1.5s, then 0.5s silence before next chord
    // This creates distinct chord sounds instead of continuous overlap

    // C Major chord (C-E-G) at 0s - plays until 1.5s
    clip.add_note(MidiNote::new(60, 100, 0.0, 1.5)); // C4
    clip.add_note(MidiNote::new(64, 100, 0.0, 1.5)); // E4
    clip.add_note(MidiNote::new(67, 100, 0.0, 1.5)); // G4

    // F Major chord (F-A-C) at 2s - plays until 3.5s
    clip.add_note(MidiNote::new(65, 100, 2.0, 1.5)); // F4
    clip.add_note(MidiNote::new(69, 100, 2.0, 1.5)); // A4
    clip.add_note(MidiNote::new(72, 100, 2.0, 1.5)); // C5

    // G Major chord (G-B-D) at 4s - plays until 5.5s
    clip.add_note(MidiNote::new(67, 100, 4.0, 1.5)); // G4
    clip.add_note(MidiNote::new(71, 100, 4.0, 1.5)); // B4
    clip.add_note(MidiNote::new(74, 100, 4.0, 1.5)); // D5

    // C Major chord at 6s - plays until 7.5s
    clip.add_note(MidiNote::new(60, 100, 6.0, 1.5)); // C4
    clip.add_note(MidiNote::new(64, 100, 6.0, 1.5)); // E4
    clip.add_note(MidiNote::new(67, 100, 6.0, 1.5)); // G4

    clip
}

fn get_chord_name(time: f64) -> &'static str {
    let t = time % 8.0;
    match t {
        0.0..=1.9 => "C Major",
        2.0..=3.9 => "F Major",
        4.0..=5.9 => "G Major",
        _ => "C Major",
    }
}

use daw_audio_engine::processor::{AudioProcessor, ProcessorConfig};

struct SimpleSynthProcessor {
    synth: Arc<Mutex<SineSynth>>,
    config: Option<ProcessorConfig>,
}

impl SimpleSynthProcessor {
    fn new(synth: Arc<Mutex<SineSynth>>) -> Self {
        Self {
            synth,
            config: None,
        }
    }
}

impl AudioProcessor for SimpleSynthProcessor {
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
        "SimpleSynth"
    }
}
