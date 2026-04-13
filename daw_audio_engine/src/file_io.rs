use crate::buffer::AudioBuffer;
use crate::error::{AudioError, AudioResult};
use hound;
use std::fs::File;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioFormat {
    Wav,
    Flac,
    Mp3,
    Ogg,
    Unknown,
}

impl AudioFormat {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Self {
        let ext = path
            .as_ref()
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());

        match ext.as_deref() {
            Some("wav") => AudioFormat::Wav,
            Some("flac") => AudioFormat::Flac,
            Some("mp3") => AudioFormat::Mp3,
            Some("ogg") => AudioFormat::Ogg,
            _ => AudioFormat::Unknown,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AudioFileInfo {
    pub sample_rate: u32,
    pub channels: usize,
    pub sample_count: usize,
    pub duration_secs: f64,
    pub format: AudioFormat,
    pub bit_depth: u16,
}

/// Load audio file and decode to interleaved f32 samples
pub fn load_audio_file<P: AsRef<Path>>(path: P) -> AudioResult<(AudioBuffer, AudioFileInfo)> {
    let format = AudioFormat::from_path(&path);

    match format {
        AudioFormat::Wav => load_wav_file(&path),
        AudioFormat::Flac | AudioFormat::Mp3 | AudioFormat::Ogg => load_symphonia_file(&path),
        AudioFormat::Unknown => Err(AudioError::Other(format!(
            "Unknown audio format: {:?}",
            path.as_ref()
        ))),
    }
}

fn load_wav_file<P: AsRef<Path>>(path: P) -> AudioResult<(AudioBuffer, AudioFileInfo)> {
    let reader = hound::WavReader::open(&path)
        .map_err(|e| AudioError::Other(format!("Failed to open WAV file: {}", e)))?;

    let spec = reader.spec();
    let sample_rate = spec.sample_rate;
    let channels = spec.channels as usize;
    let bit_depth = spec.bits_per_sample;

    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .into_samples::<f32>()
            .map(|s| s.unwrap_or(0.0))
            .collect(),
        hound::SampleFormat::Int => {
            let bits = spec.bits_per_sample;
            let max_val = (1i64 << (bits - 1)) as f32;
            reader
                .into_samples::<i32>()
                .map(|s| s.unwrap_or(0) as f32 / max_val)
                .collect()
        }
    };

    let sample_count = samples.len() / channels;
    let duration_secs = sample_count as f64 / sample_rate as f64;

    let info = AudioFileInfo {
        sample_rate,
        channels,
        sample_count,
        duration_secs,
        format: AudioFormat::Wav,
        bit_depth,
    };

    let buffer = AudioBuffer::from_interleaved(samples, channels);

    log::info!(
        "Loaded WAV: {} Hz, {}ch, {}s, {} samples",
        sample_rate,
        channels,
        duration_secs,
        sample_count
    );

    Ok((buffer, info))
}

fn load_symphonia_file<P: AsRef<Path>>(path: P) -> AudioResult<(AudioBuffer, AudioFileInfo)> {
    use symphonia::core::audio::{AudioBufferRef, Signal};
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file =
        File::open(&path).map_err(|e| AudioError::Other(format!("Failed to open file: {}", e)))?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let hint = Hint::new();
    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();
    let decoder_opts = DecoderOptions::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .map_err(|e| AudioError::Other(format!("Probe error: {}", e)))?;

    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| AudioError::Other("No audio track found".to_string()))?;

    let track_id = track.id;
    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| AudioError::Other("No sample rate".to_string()))?;
    let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(2);

    let mut sample_buf: Vec<f32> = Vec::new();

    // Create decoder
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &decoder_opts)
        .map_err(|e| AudioError::Other(format!("Decoder error: {}", e)))?;

    // Decode all packets
    while let Ok(packet) = format.next_packet() {
        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                match decoded {
                    AudioBufferRef::F32(buf) => {
                        // Symphonia stores audio in separate channels, interleave them
                        let frames = buf.frames();
                        for frame_idx in 0..frames {
                            for ch in 0..buf.spec().channels.count() {
                                sample_buf.push(buf.chan(ch)[frame_idx]);
                            }
                        }
                    }
                    AudioBufferRef::S32(buf) => {
                        let frames = buf.frames();
                        for frame_idx in 0..frames {
                            for ch in 0..buf.spec().channels.count() {
                                let sample = buf.chan(ch)[frame_idx];
                                sample_buf.push(sample as f32 / i32::MAX as f32);
                            }
                        }
                    }
                    _ => {
                        // Other formats - convert via f32
                    }
                }
            }
            Err(_) => continue,
        }
    }

    let sample_count = sample_buf.len() / channels;
    let duration_secs = sample_count as f64 / sample_rate as f64;

    let info = AudioFileInfo {
        sample_rate,
        channels,
        sample_count,
        duration_secs,
        format: AudioFormat::from_path(&path),
        bit_depth: 32,
    };

    let buffer = AudioBuffer::from_interleaved(sample_buf, channels);

    log::info!(
        "Loaded {:?}: {} Hz, {}ch, {}s",
        info.format,
        sample_rate,
        channels,
        duration_secs
    );

    Ok((buffer, info))
}

/// Save audio buffer to WAV file
pub fn save_wav_file<P: AsRef<Path>>(
    path: P,
    buffer: &AudioBuffer,
    sample_rate: u32,
) -> AudioResult<()> {
    let spec = hound::WavSpec {
        channels: buffer.channels() as u16,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = hound::WavWriter::create(&path, spec)
        .map_err(|e| AudioError::Other(format!("Failed to create WAV: {}", e)))?;

    for sample in buffer.as_interleaved() {
        writer
            .write_sample(*sample)
            .map_err(|e| AudioError::Other(format!("Write error: {}", e)))?;
    }

    writer
        .finalize()
        .map_err(|e| AudioError::Other(format!("Finalize error: {}", e)))?;

    log::info!("Saved WAV file: {:?}", path.as_ref());

    Ok(())
}

/// Resample audio buffer to target sample rate (simple linear interpolation)
pub fn resample(buffer: &AudioBuffer, from_rate: u32, to_rate: u32) -> AudioBuffer {
    if from_rate == to_rate {
        return AudioBuffer::from_interleaved(buffer.as_interleaved().to_vec(), buffer.channels());
    }

    let ratio = to_rate as f64 / from_rate as f64;
    let new_frames = (buffer.frames() as f64 * ratio) as usize;
    let channels = buffer.channels();

    let mut resampled = AudioBuffer::new(channels, new_frames);

    for ch in 0..channels {
        for new_frame in 0..new_frames {
            let old_pos = new_frame as f64 / ratio;
            let old_frame = old_pos as usize;
            let frac = old_pos.fract();

            let sample = if old_frame + 1 < buffer.frames() {
                let s1 = buffer.sample(ch, old_frame);
                let s2 = buffer.sample(ch, old_frame + 1);
                s1 + (s2 - s1) as f32 * frac as f32
            } else {
                buffer.sample(ch, buffer.frames() - 1)
            };

            resampled.set_sample(ch, new_frame, sample);
        }
    }

    resampled
}
