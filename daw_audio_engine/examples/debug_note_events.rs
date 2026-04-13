use daw_audio_engine::midi_event::MidiNote;
use daw_audio_engine::{AudioEngine, MidiClip, SineSynth, Timeline, TransportState};
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== DEBUG: Note On/Off Events ===\n");

    let mut engine = AudioEngine::new()?;
    let sample_rate = engine
        .current_device()
        .map(|d| d.sample_rate)
        .unwrap_or(48000);

    // Create synth with debug
    let mut synth = SineSynth::new(16, sample_rate);
    synth.set_gain(0.8);
    let synth = Arc::new(Mutex::new(synth));
    let synth_clone = synth.clone();

    let processor = DebugSynthProcessor::new(synth);
    engine.set_processor(Box::new(processor));

    engine.start()?;

    // Create timeline
    let mut timeline = Timeline::with_bpm("Test", sample_rate, 120.0);
    let clip = create_test_clip();

    println!("Notes in clip:");
    for note in &clip.notes {
        println!(
            "  Pitch {}: starts {:.1}s, ends {:.1}s (duration {:.1}s)",
            note.pitch,
            note.start_time,
            note.start_time + note.duration,
            note.duration
        );
    }
    println!();

    let mut track = daw_audio_engine::MidiTrack::new(
        "Test",
        std::sync::Arc::new(daw_audio_engine::SimpleSynth::new(4)),
    );
    track.add_clip(clip);
    timeline.sequencer.add_track(track);

    timeline.sequencer.transport_mut().set_loop(true);
    timeline.sequencer.transport_mut().set_loop_range(0.0, 8.0);

    println!("Starting playback... Watch for Note ON vs Note OFF\n");
    std::thread::sleep(Duration::from_millis(500));
    timeline.play();

    let start = Instant::now();
    let mut last_update = Instant::now();
    let mut on_count = 0u64;
    let mut off_count = 0u64;

    loop {
        let transport = timeline.sequencer.transport();
        let pos = transport.position();
        let state = transport.state();

        if state == TransportState::Playing {
            let events = timeline.process(256);

            for event in &events {
                match event.event_type {
                    daw_audio_engine::midi_event::MidiEventType::NoteOn { pitch, velocity } => {
                        on_count += 1;
                        println!("[ON ] Pitch {:3} vel={:3} at {:.2}s", pitch, velocity, pos);
                        synth_clone.lock().unwrap().process_event(event);
                    }
                    daw_audio_engine::midi_event::MidiEventType::NoteOff { pitch } => {
                        off_count += 1;
                        println!("[OFF] Pitch {:3}       at {:.2}s", pitch, pos);
                        synth_clone.lock().unwrap().process_event(event);
                    }
                    _ => {}
                }
            }
        }

        if last_update.elapsed() >= Duration::from_millis(500) {
            let voices = synth_clone.lock().unwrap().active_voice_count();
            print!(
                "\r[{}] {:.1}s | ON:{} OFF:{} | {} voices        ",
                if state == TransportState::Playing {
                    "PLAY"
                } else {
                    "STOP"
                },
                pos,
                on_count,
                off_count,
                voices
            );
            io::stdout().flush()?;
            last_update = Instant::now();
        }

        let mut buf = [0u8; 1];
        if io::stdin().read(&mut buf).is_ok() {
            if buf[0] as char == 'q' {
                break;
            }
        }

        if start.elapsed() > Duration::from_secs(20) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    timeline.stop();
    engine.stop()?;
    println!(
        "\n\nDone! Total: {} NoteOn, {} NoteOff",
        on_count, off_count
    );

    Ok(())
}

fn create_test_clip() -> MidiClip {
    let mut clip = MidiClip::new("Test", 0.0, 8.0);

    // Two simple notes to test
    clip.add_note(MidiNote::new(60, 100, 0.0, 1.0)); // C4 for 1 second
    clip.add_note(MidiNote::new(64, 100, 2.0, 1.0)); // E4 for 1 second

    clip
}

use daw_audio_engine::processor::{AudioProcessor, ProcessorConfig};

struct DebugSynthProcessor {
    synth: Arc<Mutex<SineSynth>>,
    config: Option<ProcessorConfig>,
}

impl DebugSynthProcessor {
    fn new(synth: Arc<Mutex<SineSynth>>) -> Self {
        Self {
            synth,
            config: None,
        }
    }
}

impl AudioProcessor for DebugSynthProcessor {
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
        "DebugSynth"
    }
}
