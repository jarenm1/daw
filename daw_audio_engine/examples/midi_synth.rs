use daw_audio_engine::{AudioEngine, InstrumentProcessor, MidiEvent, MidiInput, SimpleSynth};
use std::io::{self, IsTerminal, Write};
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== MIDI Synthesizer Demo ===\n");

    // Initialize audio engine
    println!("Starting audio engine...");
    let mut engine = daw_audio_engine::AudioEngine::new()?;
    engine.start()?;
    println!(
        "  ✓ Audio engine running at {} Hz",
        engine
            .current_device()
            .map(|d| d.sample_rate)
            .unwrap_or(48000)
    );

    // Create synth and processor
    let synth = Box::new(SimpleSynth::new(8)); // 8-voice polyphony
    let (processor, preview) = InstrumentProcessor::new(synth);
    engine.set_processor(Box::new(processor));

    // Set up MIDI input
    println!("\n--- MIDI Input ---");
    let mut midi_input = MidiInput::new();

    // List available MIDI ports
    let ports = MidiInput::list_ports();
    if ports.is_empty() {
        println!("  ⚠ No MIDI input devices found");
        println!("    You can still use keyboard preview (computer keyboard)");
    } else {
        println!("  Available MIDI ports:");
        for (i, name) in &ports {
            println!("    [{}] {}", i, name);
        }

        // Try to connect to first available port
        if let Err(e) = midi_input.connect_first_available() {
            println!("  ⚠ Could not connect to MIDI: {}", e);
        } else {
            println!("  ✓ Connected to MIDI input");
        }
    }

    // Instructions
    println!("\n=== Controls ===");
    if !ports.is_empty() && midi_input.is_connected() {
        println!("Play notes on your MIDI keyboard!");
    }
    println!("Computer keyboard:");
    println!("  A S D F G H J K L  = C4 D4 E4 F4 G4 A4 B4 C5 D5");
    println!("  W E   T Y U        = C#4 D#4  F#4 G#4 A#4");
    println!("Press Q to quit\n");

    // Main loop - simulate piano from computer keyboard
    let start_time = Instant::now();

    loop {
        print!(
            "\rTime: {:.1}s | Notes playing | Press keys...   ",
            start_time.elapsed().as_secs_f32()
        );
        io::stdout().flush()?;

        // Check for keyboard input (non-blocking)
        if io::stdin().is_terminal() {
            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_ok() {
                let key = input.trim();

                // Map keys to MIDI pitches
                let pitch = match key {
                    "a" | "A" => Some(60), // C4
                    "w" | "W" => Some(61), // C#4
                    "s" | "S" => Some(62), // D4
                    "e" | "E" => Some(63), // D#4
                    "d" | "D" => Some(64), // E4
                    "f" | "F" => Some(65), // F4
                    "t" | "T" => Some(66), // F#4
                    "g" | "G" => Some(67), // G4
                    "y" | "Y" => Some(68), // G#4
                    "h" | "H" => Some(69), // A4
                    "u" | "U" => Some(70), // A#4
                    "j" | "J" => Some(71), // B4
                    "k" | "K" => Some(72), // C5
                    "l" | "L" => Some(74), // D5
                    "q" | "Q" => break,
                    _ => None,
                };

                if let Some(p) = pitch {
                    println!(
                        "\n▶ Playing note: {} (MIDI pitch {})",
                        key.to_uppercase(),
                        p
                    );
                    preview.preview_note(p, 100); // Velocity 100

                    // Auto-release after 500ms
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_millis(500));
                        // Note: In a real app we'd send NoteOff, but for this demo
                        // the envelope will fade out naturally
                    });
                }
            }
        }

        // Check MIDI hardware input
        let events = midi_input.recv_events();
        for event in events {
            match event.event_type {
                daw_audio_engine::midi_event::MidiEventType::NoteOn { pitch, velocity } => {
                    if velocity > 0 {
                        preview.preview_note(pitch, velocity);
                        println!("\n🎹 MIDI Note On: pitch={}, vel={}", pitch, velocity);
                    }
                }
                daw_audio_engine::midi_event::MidiEventType::NoteOff { pitch } => {
                    preview.stop_note(pitch);
                }
                _ => {}
            }
        }

        std::thread::sleep(Duration::from_millis(10));

        if start_time.elapsed() > Duration::from_secs(60) {
            println!("\n\nAuto-stopping after 60 seconds");
            break;
        }
    }

    println!("\n\nStopping...");
    engine.stop()?;
    midi_input.disconnect();
    println!("Demo complete!");

    Ok(())
}
