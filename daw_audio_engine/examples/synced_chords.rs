use crossbeam::channel::{bounded, Sender};
use daw_audio_engine::midi_event::MidiEvent;
use daw_audio_engine::midi_event::MidiNote;
use daw_audio_engine::transport::Transport;
use daw_audio_engine::{AudioEngine, MidiClip, TimeSignature, Timeline, TransportState};
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== SYNCED Chord Demo ===\n");

    let mut engine = AudioEngine::new()?;
    let sample_rate = engine
        .current_device()
        .map(|d| d.sample_rate)
        .unwrap_or(48000);

    // Create timeline with clips
    let mut timeline = Timeline::with_bpm("Chords", sample_rate, 120.0);
    let clip = create_chords();

    let mut track = daw_audio_engine::MidiTrack::new(
        "Synth",
        std::sync::Arc::new(daw_audio_engine::SimpleSynth::new(4)),
    );
    track.add_clip(clip);
    timeline.sequencer.add_track(track);

    // Create transport that will be shared with audio thread
    let transport = Arc::new(AtomicU64::new(0)); // Position in samples
    let transport_display = transport.clone();

    // Create clips for the processor
    let clips = vec![create_chords()];

    // Create processor with transport and clips
    let processor = SyncedProcessor::new(sample_rate, clips, transport);
    engine.set_processor(Box::new(processor));

    engine.start()?;
    println!("✓ Audio engine started at {} Hz\n", sample_rate);
    println!("Chord Progression:");
    println!("  0-1.5s: C Major");
    println!("  2-3.5s: F Major");
    println!("  4-5.5s: G Major");
    println!("  6-7.5s: C Major\n");

    println!("Controls: p=play/pause, s=stop, q=quit");
    println!("Starting...\n");
    std::thread::sleep(Duration::from_millis(500));

    // Start timeline for display only (not for event generation)
    let mut display_timeline = Timeline::with_bpm("Display", sample_rate, 120.0);
    display_timeline.sequencer.transport_mut().set_loop(true);
    display_timeline
        .sequencer
        .transport_mut()
        .set_loop_range(0.0, 8.0);
    display_timeline.play();

    let start = Instant::now();
    let mut last_update = Instant::now();
    let mut last_chord = String::new();
    let mut is_playing = true;

    loop {
        let pos = display_timeline.sequencer.transport().position();

        // Update display
        if last_update.elapsed() >= Duration::from_millis(100) {
            let chord = get_chord(pos);
            let status = if is_playing { "▶" } else { "■" };

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

        if is_playing {
            display_timeline.process(256); // Just for display timing
        }

        // Input
        let mut buf = [0u8; 1];
        if io::stdin().read(&mut buf).is_ok() {
            match buf[0] as char {
                'p' => {
                    is_playing = !is_playing;
                    if is_playing {
                        display_timeline.play();
                    } else {
                        display_timeline.pause();
                    }
                }
                's' => {
                    display_timeline.stop();
                    display_timeline.seek(0.0);
                    is_playing = false;
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

    display_timeline.stop();
    engine.stop()?;
    println!("\n\nDone!");

    Ok(())
}

fn create_chords() -> MidiClip {
    let mut clip = MidiClip::new("Chords", 0.0, 8.0);

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
use daw_audio_engine::sine_synth::SineSynth;

struct SyncedProcessor {
    synth: SineSynth,
    clips: Vec<MidiClip>,
    position: Arc<AtomicU64>,
    sample_rate: u32,
    config: Option<ProcessorConfig>,
    last_processed_sample: u64,
}

impl SyncedProcessor {
    fn new(sample_rate: u32, clips: Vec<MidiClip>, position: Arc<AtomicU64>) -> Self {
        let mut synth = SineSynth::new(8, sample_rate);
        synth.set_gain(0.7);

        Self {
            synth,
            clips,
            position,
            sample_rate,
            config: None,
            last_processed_sample: 0,
        }
    }
}

impl AudioProcessor for SyncedProcessor {
    fn process(&mut self, _input: &[f32], output: &mut [f32]) {
        let channels = self.config.as_ref().map(|c| c.output_channels).unwrap_or(2);
        let frames = output.len() / channels;

        // Get current position from atomic counter
        let current_sample = self.position.load(Ordering::Relaxed);
        let current_time = current_sample as f64 / self.sample_rate as f64;

        // Process events for this buffer
        // Generate events at the START of this buffer
        for clip in &self.clips {
            let clip_pos = current_time - clip.start_time;
            if clip_pos >= 0.0 && clip_pos < clip.duration {
                for note in &clip.notes {
                    // Check if this note should start in this buffer
                    let note_start_sample =
                        ((note.start_time + clip.start_time) * self.sample_rate as f64) as u64;
                    let note_end_sample =
                        note_start_sample + (note.duration * self.sample_rate as f64) as u64;

                    // Note on
                    if note_start_sample >= current_sample
                        && note_start_sample < current_sample + frames as u64
                    {
                        let event = MidiEvent::note_on(note.pitch, note.velocity, note.channel);
                        self.synth.process_event(&event);
                    }

                    // Note off
                    if note_end_sample >= current_sample
                        && note_end_sample < current_sample + frames as u64
                    {
                        let event = MidiEvent::note_off(note.pitch, note.channel);
                        self.synth.process_event(&event);
                    }
                }
            }
        }

        // Advance position
        self.position
            .store(current_sample + frames as u64, Ordering::Relaxed);

        // Handle looping
        if current_time >= 8.0 {
            self.position.store(0, Ordering::Relaxed);
        }

        // Render audio
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
        "SyncedProcessor"
    }
}
