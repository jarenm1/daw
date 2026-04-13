use crossbeam::channel::{bounded, Sender};
use daw_audio_engine::midi_event::MidiEvent;
use daw_audio_engine::{AudioEngine, MidiClip, SineSynth, Timeline, TransportState};
use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== Thread-Safe Chord Demo ===\n");

    let mut engine = AudioEngine::new()?;
    let sample_rate = engine
        .current_device()
        .map(|d| d.sample_rate)
        .unwrap_or(48000);

    // Create event channel
    let (event_tx, event_rx) = bounded::<MidiEvent>(1024);

    // Create processor that receives events from channel
    let processor = ChannelSynthProcessor::new(event_rx, sample_rate);
    engine.set_processor(Box::new(processor));

    engine.start()?;
    println!("✓ Audio engine started at {} Hz\n", sample_rate);

    // Create timeline
    let mut timeline = Timeline::with_bpm("Chords", sample_rate, 120.0);
    let clip = create_chords();

    let mut track = daw_audio_engine::MidiTrack::new(
        "Synth",
        std::sync::Arc::new(daw_audio_engine::SimpleSynth::new(4)),
    );
    track.add_clip(clip);
    timeline.sequencer.add_track(track);

    timeline.sequencer.transport_mut().set_loop(true);
    timeline.sequencer.transport_mut().set_loop_range(0.0, 8.0);

    println!("Chord Progression:");
    println!("  0-1.5s: C Major (C-E-G)");
    println!("  2-3.5s: F Major (F-A-C)");
    println!("  4-5.5s: G Major (G-B-D)");
    println!("  6-7.5s: C Major (C-E-G)\n");
    println!("Each chord plays for 1.5 seconds with 0.5s gap\n");

    println!("Controls: p=play/pause, s=stop, q=quit");
    println!("Starting in 1 second...\n");
    std::thread::sleep(Duration::from_secs(1));

    timeline.play();

    let start = Instant::now();
    let mut last_update = Instant::now();
    let mut last_chord = String::new();

    loop {
        let transport = timeline.sequencer.transport();
        let pos = transport.position();
        let state = transport.state();

        if state == TransportState::Playing {
            let events = timeline.process(256);

            // Send events through channel (non-blocking)
            for event in events {
                let _ = event_tx.try_send(event);
            }
        }

        if last_update.elapsed() >= Duration::from_millis(100) {
            let chord = get_chord(pos);
            let status = match state {
                TransportState::Playing => "▶",
                _ => "■",
            };

            // Only print when chord changes
            let chord_str = chord.to_string();
            if chord_str != last_chord {
                println!("\n{} Playing: {}", status, chord);
                last_chord = chord_str;
            } else {
                print!("\r{} {:.1}s | {}          ", status, pos, chord);
                io::stdout().flush()?;
            }

            last_update = Instant::now();
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

fn create_chords() -> MidiClip {
    let mut clip = MidiClip::new("Chords", 0.0, 8.0);

    // C Major at 0s
    clip.add_note(daw_audio_engine::midi_event::MidiNote::new(
        60, 100, 0.0, 1.5,
    ));
    clip.add_note(daw_audio_engine::midi_event::MidiNote::new(
        64, 100, 0.0, 1.5,
    ));
    clip.add_note(daw_audio_engine::midi_event::MidiNote::new(
        67, 100, 0.0, 1.5,
    ));

    // F Major at 2s
    clip.add_note(daw_audio_engine::midi_event::MidiNote::new(
        65, 100, 2.0, 1.5,
    ));
    clip.add_note(daw_audio_engine::midi_event::MidiNote::new(
        69, 100, 2.0, 1.5,
    ));
    clip.add_note(daw_audio_engine::midi_event::MidiNote::new(
        72, 100, 2.0, 1.5,
    ));

    // G Major at 4s
    clip.add_note(daw_audio_engine::midi_event::MidiNote::new(
        67, 100, 4.0, 1.5,
    ));
    clip.add_note(daw_audio_engine::midi_event::MidiNote::new(
        71, 100, 4.0, 1.5,
    ));
    clip.add_note(daw_audio_engine::midi_event::MidiNote::new(
        74, 100, 4.0, 1.5,
    ));

    // C Major at 6s
    clip.add_note(daw_audio_engine::midi_event::MidiNote::new(
        60, 100, 6.0, 1.5,
    ));
    clip.add_note(daw_audio_engine::midi_event::MidiNote::new(
        64, 100, 6.0, 1.5,
    ));
    clip.add_note(daw_audio_engine::midi_event::MidiNote::new(
        67, 100, 6.0, 1.5,
    ));

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

use crossbeam::channel::Receiver;
use daw_audio_engine::processor::{AudioProcessor, ProcessorConfig};

struct ChannelSynthProcessor {
    synth: SineSynth,
    event_rx: Receiver<MidiEvent>,
    config: Option<ProcessorConfig>,
}

impl ChannelSynthProcessor {
    fn new(event_rx: Receiver<MidiEvent>, sample_rate: u32) -> Self {
        let mut synth = SineSynth::new(8, sample_rate);
        synth.set_gain(0.7);

        Self {
            synth,
            event_rx,
            config: None,
        }
    }
}

impl AudioProcessor for ChannelSynthProcessor {
    fn process(&mut self, _input: &[f32], output: &mut [f32]) {
        // Process all pending MIDI events (non-blocking)
        while let Ok(event) = self.event_rx.try_recv() {
            self.synth.process_event(&event);
        }

        let channels = self.config.as_ref().map(|c| c.output_channels).unwrap_or(2);
        self.synth.render(output, channels);
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
        "ChannelSynth"
    }
}
