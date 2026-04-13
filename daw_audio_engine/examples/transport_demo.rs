use daw_audio_engine::midi_event::{MidiEvent, MidiNote};
use daw_audio_engine::{
    AudioEngine, InstrumentProcessor, MidiClip, SimpleSynth, Timeline, TransportState,
};
use std::io::{self, Write};
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== Transport & Sequencer Demo ===\n");

    // Initialize audio engine
    println!("Starting audio engine...");
    let mut engine = AudioEngine::new()?;
    let sample_rate = engine
        .current_device()
        .map(|d| d.sample_rate)
        .unwrap_or(48000);
    engine.start()?;
    println!("  ✓ Audio engine running at {} Hz\n", sample_rate);

    // Create synth and processor
    let (processor, preview_handle) = InstrumentProcessor::new(Box::new(SimpleSynth::new(16)));
    engine.set_processor(Box::new(processor));

    // Create timeline with 120 BPM
    let mut timeline = Timeline::with_bpm("Demo Song", sample_rate, 120.0);

    // Create a MIDI clip with a simple chord progression
    let clip = create_chord_progression();

    // Store clip in a simple track
    use std::sync::Arc;
    let mut track = daw_audio_engine::MidiTrack::new("Synth", Arc::new(SimpleSynth::new(16)));
    track.add_clip(clip);
    timeline.sequencer.add_track(track);

    // Display song info
    println!("Created timeline: '{}'", timeline.name);
    println!("  BPM: {}", timeline.sequencer.transport().bpm());
    println!("  Time Signature: 4/4");
    println!("  Sample rate: {} Hz", sample_rate);
    println!("  Chords: C → F → G → C");
    println!(
        "  Total duration: {:.1} seconds\n",
        timeline.sequencer.total_duration()
    );

    // Enable looping
    timeline.sequencer.transport_mut().set_loop(true);
    timeline.sequencer.transport_mut().set_loop_range(0.0, 8.0);

    // Instructions
    println!("=== Transport Controls ===");
    println!("  p = Play / Pause toggle");
    println!("  s = Stop (and rewind)");
    println!("  l = Toggle loop");
    println!("  + = Increase BPM by 5");
    println!("  - = Decrease BPM by 5");
    println!("  q = Quit\n");

    // Auto-start playing
    println!("Auto-starting in 1 second...\n");
    std::thread::sleep(Duration::from_secs(1));
    timeline.play();

    // Main control loop
    let start_time = Instant::now();
    let mut last_display_update = Instant::now();
    let mut last_position = String::new();
    let mut current_chord = String::new();

    loop {
        let transport = timeline.sequencer.transport();
        let current_pos = transport.position_formatted();
        let state = transport.state();
        let bpm = transport.bpm();
        let pos_secs = transport.position();

        // Determine current chord for display
        let chord = get_chord_at_time(pos_secs);

        // Update display every 100ms
        if last_display_update.elapsed() >= Duration::from_millis(100) {
            let status = match state {
                TransportState::Playing => "▶ PLAY",
                TransportState::Stopped => "■ STOP",
                TransportState::Paused => "⏸ PAUSE",
                TransportState::Recording => "● REC",
            };

            let display_line = format!(
                "[{}] {} | {:.0} BPM | Loop: {} | {}",
                status,
                current_pos,
                bpm,
                if transport.is_looping() { "ON" } else { "OFF" },
                chord
            );

            if display_line != last_position || chord != current_chord {
                print!("\r{}                          ", display_line);
                io::stdout().flush()?;
                last_position = display_line;
                current_chord = chord.to_string();
            }

            last_display_update = Instant::now();
        }

        // Process audio and generate MIDI events if playing
        if state == TransportState::Playing {
            let events = timeline.process(256); // Buffer size
            for event in events {
                preview_handle.send_event(event);
            }
        }

        // Check for keyboard input (non-blocking)
        use std::io::Read;
        let mut stdin = io::stdin();
        let mut buffer = [0u8; 1];

        if stdin.read(&mut buffer).is_ok() {
            let key = buffer[0] as char;

            match key.to_ascii_lowercase() {
                'p' => {
                    timeline.toggle();
                    let new_state = timeline.sequencer.transport().state();
                    println!(
                        "\n{}",
                        match new_state {
                            TransportState::Playing => "▶ Started playback",
                            TransportState::Paused => "⏸ Paused",
                            _ => "■ Stopped",
                        }
                    );
                }
                's' => {
                    timeline.stop();
                    timeline.seek(0.0);
                    println!("\n⏮ Stopped and rewound");
                    last_position.clear();
                }
                'l' => {
                    let was_looping = timeline.sequencer.transport().is_looping();
                    timeline.sequencer.transport_mut().set_loop(!was_looping);
                    println!("\n🔁 Loop {}", if was_looping { "OFF" } else { "ON" });
                }
                '+' | '=' => {
                    let new_bpm = bpm + 5.0;
                    timeline.sequencer.transport_mut().set_bpm(new_bpm);
                    println!("\n⬆ BPM: {:.0}", new_bpm);
                }
                '-' | '_' => {
                    let new_bpm = (bpm - 5.0).max(20.0);
                    timeline.sequencer.transport_mut().set_bpm(new_bpm);
                    println!("\n⬇ BPM: {:.0}", new_bpm);
                }
                'q' => {
                    println!("\n\nQuitting...");
                    break;
                }
                '\n' | '\r' => {}
                _ => {}
            }

            // Drain remaining characters
            while stdin.read(&mut buffer).is_ok() && buffer[0] != b'\n' {}
        }

        // Auto-stop after 60 seconds
        if start_time.elapsed() > Duration::from_secs(60) {
            println!("\n\nAuto-stopping after 60 seconds");
            break;
        }

        std::thread::sleep(Duration::from_millis(5));
    }

    timeline.stop();
    engine.stop()?;

    println!("\n\nDemo complete!");
    println!("You heard: C → F → G → C chord progression");

    Ok(())
}

fn create_chord_progression() -> MidiClip {
    let mut clip = MidiClip::new("Chords", 0.0, 8.0);

    // C Major chord (C-E-G) - 2 seconds each
    clip.add_note(MidiNote::new(60, 100, 0.0, 1.8)); // C4
    clip.add_note(MidiNote::new(64, 100, 0.0, 1.8)); // E4
    clip.add_note(MidiNote::new(67, 100, 0.0, 1.8)); // G4

    // F Major chord (F-A-C) at 2 seconds
    clip.add_note(MidiNote::new(65, 100, 2.0, 1.8)); // F4
    clip.add_note(MidiNote::new(69, 100, 2.0, 1.8)); // A4
    clip.add_note(MidiNote::new(72, 100, 2.0, 1.8)); // C5

    // G Major chord (G-B-D) at 4 seconds
    clip.add_note(MidiNote::new(67, 100, 4.0, 1.8)); // G4
    clip.add_note(MidiNote::new(71, 100, 4.0, 1.8)); // B4
    clip.add_note(MidiNote::new(74, 100, 4.0, 1.8)); // D5

    // C Major chord at 6 seconds
    clip.add_note(MidiNote::new(60, 100, 6.0, 1.8)); // C4
    clip.add_note(MidiNote::new(64, 100, 6.0, 1.8)); // E4
    clip.add_note(MidiNote::new(67, 100, 6.0, 1.8)); // G4

    clip
}

fn get_chord_at_time(time: f64) -> &'static str {
    let time = time % 8.0; // Loop every 8 seconds
    match time {
        0.0..=1.9 => "C Major",
        2.0..=3.9 => "F Major",
        4.0..=5.9 => "G Major",
        6.0..=7.9 => "C Major",
        _ => "",
    }
}
