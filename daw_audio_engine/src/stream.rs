use crate::error::{AudioError, AudioResult};
use cpal::{
    traits::{DeviceTrait, StreamTrait},
    BufferSize, Device, Sample, SampleFormat, SampleRate, Stream, StreamConfig,
};
use crossbeam::channel::{bounded, Receiver, Sender};
use log::{error, info};
use parking_lot::Mutex;
use std::sync::Arc;

pub type AudioCallback = Box<dyn FnMut(&[f32], &mut [f32]) + Send + 'static>;

pub struct AudioStream {
    stream: Stream,
    config: StreamConfig,
    sample_format: SampleFormat,
    is_running: Arc<Mutex<bool>>,
    latency_ms: f32,
}

impl AudioStream {
    pub fn new_output(
        device: &Device,
        config: &StreamConfig,
        sample_format: SampleFormat,
        mut callback: AudioCallback,
    ) -> AudioResult<Self> {
        let channels = config.channels as usize;
        let latency_ms = Self::calculate_latency_ms(config);

        let stream = match sample_format {
            SampleFormat::F32 => device.build_output_stream(
                config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let frames = data.len() / channels;
                    let input = vec![0.0f32; frames * channels];
                    callback(&input, data);
                },
                |err| error!("Stream error: {}", err),
                None,
            )?,
            SampleFormat::I16 => device.build_output_stream(
                config,
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    let frames = data.len() / channels;
                    let input = vec![0.0f32; frames * channels];
                    let mut output_float = vec![0.0f32; data.len()];
                    callback(&input, &mut output_float);
                    for (i, sample) in output_float.iter().enumerate() {
                        data[i] = Sample::from_sample(*sample);
                    }
                },
                |err| error!("Stream error: {}", err),
                None,
            )?,
            SampleFormat::U16 => device.build_output_stream(
                config,
                move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                    let frames = data.len() / channels;
                    let input = vec![0.0f32; frames * channels];
                    let mut output_float = vec![0.0f32; data.len()];
                    callback(&input, &mut output_float);
                    for (i, sample) in output_float.iter().enumerate() {
                        data[i] = Sample::from_sample(*sample);
                    }
                },
                |err| error!("Stream error: {}", err),
                None,
            )?,
            _ => {
                return Err(AudioError::Other(format!(
                    "Unsupported sample format: {:?}",
                    sample_format
                )))
            }
        };

        let is_running = Arc::new(Mutex::new(false));

        Ok(Self {
            stream,
            config: config.clone(),
            sample_format,
            is_running,
            latency_ms,
        })
    }

    pub fn new_duplex(
        input_device: &Device,
        output_device: &Device,
        config: &StreamConfig,
        sample_format: SampleFormat,
        mut callback: AudioCallback,
    ) -> AudioResult<Self> {
        let channels = config.channels as usize;
        let latency_ms = Self::calculate_latency_ms(config);

        let (input_tx, input_rx): (Sender<Vec<f32>>, Receiver<Vec<f32>>) = bounded(2);
        let (_output_tx, _output_rx): (Sender<Vec<f32>>, Receiver<Vec<f32>>) = bounded(2);

        let _input_stream = input_device.build_input_stream(
            config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let _ = input_tx.try_send(data.to_vec());
            },
            |err| error!("Input stream error: {}", err),
            None,
        )?;

        let output_stream = output_device.build_output_stream(
            config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                if let Ok(input_data) = input_rx.try_recv() {
                    callback(&input_data, data);
                } else {
                    let frames = data.len() / channels;
                    let input = vec![0.0f32; frames * channels];
                    callback(&input, data);
                }
            },
            |err| error!("Output stream error: {}", err),
            None,
        )?;

        let is_running = Arc::new(Mutex::new(false));

        Ok(Self {
            stream: output_stream,
            config: config.clone(),
            sample_format,
            is_running,
            latency_ms,
        })
    }

    pub fn start(&self) -> AudioResult<()> {
        self.stream.play()?;
        *self.is_running.lock() = true;
        info!("Audio stream started - latency: {:.1}ms", self.latency_ms);
        Ok(())
    }

    pub fn stop(&self) -> AudioResult<()> {
        self.stream.pause()?;
        *self.is_running.lock() = false;
        info!("Audio stream stopped");
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        *self.is_running.lock()
    }

    pub fn latency_ms(&self) -> f32 {
        self.latency_ms
    }

    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate.0
    }

    pub fn buffer_size(&self) -> u32 {
        match self.config.buffer_size {
            BufferSize::Fixed(size) => size,
            BufferSize::Default => 512,
        }
    }

    fn calculate_latency_ms(config: &StreamConfig) -> f32 {
        let buffer_size = match config.buffer_size {
            BufferSize::Fixed(size) => size as f32,
            BufferSize::Default => 512.0,
        };
        let sample_rate = config.sample_rate.0 as f32;
        (buffer_size / sample_rate) * 1000.0
    }
}

pub fn get_low_latency_config(
    device: &Device,
    preferred_sample_rate: Option<u32>,
    preferred_buffer_size: Option<u32>,
) -> AudioResult<(StreamConfig, SampleFormat)> {
    let default_config = device.default_output_config()?;
    let sample_format = default_config.sample_format();
    let sample_rate = preferred_sample_rate.unwrap_or(default_config.sample_rate().0);
    let buffer_size = preferred_buffer_size.unwrap_or(256);

    let config = StreamConfig {
        channels: default_config.channels(),
        sample_rate: SampleRate(sample_rate),
        buffer_size: BufferSize::Fixed(buffer_size),
    };

    Ok((config, sample_format))
}
