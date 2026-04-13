use daw_audio_engine::midi_event::MidiNote;
use daw_audio_engine::{AudioEngine, MidiClip, SineSynth, Timeline, TransportState};
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== DEBUG: MIDI Sequencer ===\n");

    let mut engine = AudioEngine::new()?;
    let sample_rate = engine
        .current_device()
        .map(|d| d.sample_rate)
        .unwrap_or(48000);

    // Create synth with proper volume
    let mut synth_instance = SineSynth::new(16, sample_rate);
    synth_instance.set_gain(0.8); // 80% volume (default was 30%)
    let synth = Arc::new(Mutex::new(synth_instance));
    let synth_for_processor = synth.clone();
    let synth_for_debug = synth.clone();

    // Create processor
    let processor = SimpleSynthProcessor::new(synth_for_processor);
    engine.set_processor(Box::new(processor));

    engine.start()?;
    println!("✓ Audio started at {} Hz\n", sample_rate);

    // Create timeline
    let mut timeline = Timeline::with_bpm("Test", sample_rate, 120.0);
    let clip = create_test_clip();

    // Debug: Print all notes in clip
    println!("Notes in clip:");
    for note in &clip.notes {
        println!(
            "  Pitch {} at {:.1}s for {:.1}s",
            note.pitch, note.start_time, note.duration
        );
    }
    println!();

    // Add to track
    let mut track = daw_audio_engine::MidiTrack::new(
        "Test",
        std::sync::Arc::new(daw_audio_engine::SimpleSynth::new(4)),
    );
    track.add_clip(clip);
    timeline.sequencer.add_track(track);

    // Enable loop
    timeline.sequencer.transport_mut().set_loop(true);
    timeline.sequencer.transport_mut().set_loop_range(0.0, 8.0);

    println!("Starting in 1 second...");
    println!("Each chord should play for ~2 seconds");
    println!("Controls: p=play/pause, s=stop, q=quit\n");

    std::thread::sleep(Duration::from_secs(1));
    timeline.play();

    let start = Instant::now();
    let mut last_update = Instant::now();
    let mut event_count = 0u64;

    loop {
        let transport = timeline.sequencer.transport();
        let pos = transport.position();
        let state = transport.state();

        // Process every 100ms
        if state == TransportState::Playing {
            let events = timeline.process(256);

            // Debug: Show events
            if !events.is_empty() {
                for event in &events {
                    match event.event_type {
                        daw_audio_engine::midi_event::MidiEventType::NoteOn { pitch, velocity } => {
                            println!(
                                "  [EVENT] Note ON: pitch={} vel={} at {:.2}s",
                                pitch, velocity, pos
                            );
                        }
                        daw_audio_engine::midi_event::MidiEventType::NoteOff { pitch } => {
                            println!("  [EVENT] Note OFF: pitch={} at {:.2}s", pitch, pos);
                        }
                        _ => {}
                    }
                }
                event_count += events.len() as u64;

                // Send to synth
                for event in &events {
                    synth_for_debug.lock().unwrap().process_event(event);
                }
            }
        }

        // Display
        if last_update.elapsed() >= Duration::from_millis(200) {
            let voices = synth_for_debug.lock().unwrap().active_voice_count();
            print!(
                "\r[{}] {:.1}s | {} events | {} voices      ",
                if state == TransportState::Playing {
                    "PLAY"
                } else {
                    "STOP"
                },
                pos,
                event_count,
                voices
            );
            io::stdout().flush()?;
            last_update = Instant::now();
        }

        // Input
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

fn create_test_clip() -> MidiClip {
    let mut clip = MidiClip::new("Chords", 0.0, 8.0);

    // C Major chord (C-E-G) at 0s
    clip.add_note(MidiNote::new(60, 100, 0.0, 1.8)); // C4
    clip.add_note(MidiNote::new(64, 100, 0.0, 1.8)); // E4
    clip.add_note(MidiNote::new(67, 100, 0.0, 1.8)); // G4

    // F Major chord (F-A-C) at 2s
    clip.add_note(MidiNote::new(65, 100, 2.0, 1.8)); // F4
    clip.add_note(MidiNote::new(69, 100, 2.0, 1.8)); // A4
    clip.add_note(MidiNote::new(72, 100, 2.0, 1.8)); // C5

    // G Major chord (G-B-D) at 4s
    clip.add_note(MidiNote::new(67, 100, 4.0, 1.8)); // G4
    clip.add_note(MidiNote::new(71, 100, 4.0, 1.8)); // B4
    clip.add_note(MidiNote::new(74, 100, 4.0, 1.8)); // D5

    // C Major chord at 6s
    clip.add_note(MidiNote::new(60, 100, 6.0, 1.8)); // C4
    clip.add_note(MidiNote::new(64, 100, 6.0, 1.8)); // E4
    clip.add_note(MidiNote::new(67, 100, 6.0, 1.8)); // G4

    clip
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
