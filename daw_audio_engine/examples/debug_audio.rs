use daw_audio_engine::buffer::AudioBuffer;
use daw_audio_engine::file_io::save_wav_file;
use daw_audio_engine::{AudioClip, AudioEngine, AudioPlayer, ClipPlayerProcessor};
use std::f32::consts::PI;
use std::io::{self, Write};
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== Audio Debug Test ===\n");

    // Generate test tone
    let test_file = "/tmp/test_tone.wav";
    generate_test_tone(test_file)?;
    println!("✓ Generated test tone: {}", test_file);

    // Load it
    let clip = AudioClip::from_file(test_file)?;
    let info = clip.info().clone();
    println!(
        "✓ Loaded: {} Hz, {}ch, {:.1}s",
        info.sample_rate, info.channels, info.duration_secs
    );

    // Test player directly (no engine) - render to file
    println!("\n--- Testing AudioPlayer ---");
    let mut player = AudioPlayer::new(info.channels);
    player.load_clip(clip.clone());
    player.set_gain(1.0);

    // Render 1 second to a buffer
    let _sample_rate = info.sample_rate as usize;
    let test_duration = 1.0f64;
    let test_samples = (test_duration * info.sample_rate as f64) as usize * info.channels;
    let mut test_output = vec![0.0f32; test_samples];

    println!("Rendering {} samples...", test_samples);
    player.play();
    player.process(&mut test_output);

    // Check if we got non-zero samples
    let max_sample = test_output.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let non_zero_count = test_output.iter().filter(|&&s| s != 0.0).count();

    println!("  Max sample value: {:.6}", max_sample);
    println!(
        "  Non-zero samples: {} / {}",
        non_zero_count,
        test_output.len()
    );

    if max_sample > 0.001 {
        println!("  ✓ Audio is being generated!");
    } else {
        println!("  ✗ No audio output detected");
    }

    // Save rendered output to verify
    let output_file = "/tmp/rendered_output.wav";
    let output_buffer = AudioBuffer::from_interleaved(test_output, info.channels);
    save_wav_file(output_file, &output_buffer, info.sample_rate)?;
    println!("  ✓ Rendered output saved to: {}", output_file);

    // Now test with engine
    println!("\n--- Testing with AudioEngine ---");
    let mut engine = AudioEngine::new()?;
    let engine_rate = engine
        .current_device()
        .map(|d| d.sample_rate)
        .unwrap_or(48000);
    println!("Engine sample rate: {} Hz", engine_rate);

    // Resample if needed
    if info.sample_rate != engine_rate {
        println!("  Resampling clip to {} Hz...", engine_rate);
        clip.resample_to(engine_rate);
    }

    let channels = info.channels;
    let (processor, player_handle) = ClipPlayerProcessor::with_clip(clip, channels);
    engine.set_processor(Box::new(processor));

    println!("Starting engine...");
    engine.start()?;
    println!("  ✓ Engine started");

    // Check player state
    println!("\n  Player state before play():");
    println!("    Position: {:.3}s", player_handle.position_secs());
    println!("    Duration: {:.1}s", player_handle.duration_secs());

    // Start playback
    player_handle.set_gain(0.8);
    player_handle.set_loop(true);
    player_handle.play();
    println!("\n  ▶ Started playback (Gain: 80%, Loop: ON)");

    // Monitor for 5 seconds
    println!("\nMonitoring for 5 seconds...");
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        print!(
            "\r  Position: {:.2}s / {:.1}s    ",
            player_handle.position_secs(),
            player_handle.duration_secs()
        );
        io::stdout().flush()?;
        std::thread::sleep(Duration::from_millis(100));
    }

    println!("\n\nStopping...");
    player_handle.stop();
    engine.stop()?;
    println!("  ✓ Engine stopped");

    // Check final state
    println!("\nFinal player state:");
    println!("  Position: {:.3}s", player_handle.position_secs());

    // Cleanup
    let _ = std::fs::remove_file(test_file);
    // Keep rendered file for verification

    println!("\n=== Debug Complete ===");
    println!("If you didn't hear audio:");
    println!("  1. Check system volume");
    println!("  2. Check 'pavucontrol' or audio mixer");
    println!("  3. Verify rendered_output.wav plays correctly");
    println!("  4. Try: aplay /tmp/rendered_output.wav");

    Ok(())
}

fn generate_test_tone(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let sample_rate = 48000u32;
    let duration = 3.0f64;
    let frequency = 440.0f32;
    let channels = 2usize;

    let total_samples = (duration * sample_rate as f64) as usize;
    let mut buffer = AudioBuffer::new(channels, total_samples);

    for i in 0..total_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (t * frequency * 2.0 * PI).sin() * 0.8; // 80% amplitude

        for ch in 0..channels {
            buffer.set_sample(ch, i, sample);
        }
    }

    save_wav_file(path, &buffer, sample_rate)?;
    Ok(())
}
