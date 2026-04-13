use daw_audio_engine::buffer::AudioBuffer;
use daw_audio_engine::file_io::save_wav_file;
use daw_audio_engine::{AudioClip, AudioEngine, ClipPlayerProcessor};
use std::f32::consts::PI;
use std::io::{self, Write};
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== Audio Playback Demo ===\n");

    // Generate a test audio file (440Hz sine wave)
    let test_file = "/tmp/test_tone.wav";
    generate_test_tone(test_file)?;
    println!("Generated test tone: {}\n", test_file);

    // Load the audio file
    println!("Loading audio file...");
    let clip = AudioClip::from_file(test_file)?;
    let info = clip.info().clone();
    println!("  Sample rate: {} Hz", info.sample_rate);
    println!("  Channels: {}", info.channels);
    println!("  Duration: {:.2} seconds", info.duration_secs);
    println!("  Samples: {}\n", info.sample_count);

    // Start audio engine
    println!("Starting audio engine...");
    let mut engine = AudioEngine::new()?;

    // Resample clip if needed to match engine sample rate
    let engine_sample_rate = engine
        .current_device()
        .map(|d| d.sample_rate)
        .unwrap_or(48000);

    if info.sample_rate != engine_sample_rate {
        println!(
            "  Resampling clip from {} Hz to {} Hz...",
            info.sample_rate, engine_sample_rate
        );
        clip.resample_to(engine_sample_rate);
    }

    // Create player processor
    let channels = info.channels;
    let (processor, player) = ClipPlayerProcessor::with_clip(clip, channels);

    // Set up the engine with our player
    engine.set_processor(Box::new(processor));

    engine.start()?;
    println!("  Audio engine running at {} Hz\n", engine_sample_rate);

    println!("Press Enter to start playback...");
    io::stdin().read_line(&mut String::new())?;

    // Configure and start playback
    player.set_gain(0.5); // 50% volume
    player.set_loop(true);
    player.play();

    println!("▶ Playing!");
    println!("  Duration: {:.1}s", player.duration_secs());
    println!("  Looping: true");
    println!("  Volume: 50%\n");

    // Monitor playback for 10 seconds
    let start_time = Instant::now();
    loop {
        print!(
            "\rTime: {:.1}s | Position: {:.1}s / {:.1}s | Vol: 50%   ",
            start_time.elapsed().as_secs_f32(),
            player.position_secs(),
            player.duration_secs()
        );
        io::stdout().flush()?;

        std::thread::sleep(Duration::from_millis(100));
        if start_time.elapsed() > Duration::from_secs(10) {
            break;
        }
    }

    println!("\n\nStopping...");
    player.stop();
    engine.stop()?;
    println!("Audio engine stopped.");
    println!("Demo complete!");

    // Cleanup
    let _ = std::fs::remove_file(test_file);

    Ok(())
}

fn generate_test_tone(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let sample_rate = 48000u32;
    let duration = 3.0f64; // 3 seconds
    let frequency = 440.0f32; // A4 note
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
