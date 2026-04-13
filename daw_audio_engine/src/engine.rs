use crate::device::{AudioDeviceInfo, DeviceManager};
use crate::error::{AudioError, AudioResult};
use crate::processor::AudioProcessor;
use crate::stream::{get_low_latency_config, AudioCallback, AudioStream};
use crate::{DEFAULT_BUFFER_SIZE, DEFAULT_SAMPLE_RATE, TARGET_LATENCY_MS};
use cpal::traits::{DeviceTrait, HostTrait};
use log::info;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct AudioEngine {
    device_manager: DeviceManager,
    active_stream: Arc<Mutex<Option<AudioStream>>>,
    processor: Arc<Mutex<Option<Box<dyn AudioProcessor>>>>,
    current_device: Arc<Mutex<Option<AudioDeviceInfo>>>,
    current_config: EngineConfig,
}

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub sample_rate: u32,
    pub buffer_size: u32,
    pub channels: u16,
    pub input_enabled: bool,
    pub output_enabled: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            sample_rate: DEFAULT_SAMPLE_RATE,
            buffer_size: DEFAULT_BUFFER_SIZE,
            channels: 2,
            input_enabled: false,
            output_enabled: true,
        }
    }
}

impl AudioEngine {
    pub fn new() -> AudioResult<Self> {
        let device_manager = DeviceManager::new();
        let default_device = device_manager.get_default_output().ok();

        if let Some(ref device) = default_device {
            info!("Default audio device: {}", device.name);
        }

        Ok(Self {
            device_manager,
            active_stream: Arc::new(Mutex::new(None)),
            processor: Arc::new(Mutex::new(None)),
            current_device: Arc::new(Mutex::new(default_device)),
            current_config: EngineConfig::default(),
        })
    }

    pub fn list_devices(&self) -> AudioResult<Vec<AudioDeviceInfo>> {
        self.device_manager.enumerate_devices()
    }

    pub fn find_asio_devices(&self) -> AudioResult<Vec<AudioDeviceInfo>> {
        let all_devices = self.list_devices()?;
        Ok(all_devices.into_iter().filter(|d| d.is_asio).collect())
    }

    pub fn set_processor(&self, processor: Box<dyn AudioProcessor>) {
        *self.processor.lock() = Some(processor);
    }

    pub fn start_with_device(
        &mut self,
        device_name: &str,
        config: EngineConfig,
    ) -> AudioResult<()> {
        self.stop()?;

        let device_info = self.device_manager.find_device_by_name(device_name)?;
        info!("Starting engine with device: {}", device_info);

        let host = cpal::default_host();
        let device = if device_info.is_output {
            host.output_devices()?
                .find(|d| d.name().map(|n| n == device_info.name).unwrap_or(false))
        } else {
            host.input_devices()?
                .find(|d| d.name().map(|n| n == device_info.name).unwrap_or(false))
        }
        .ok_or_else(|| AudioError::DeviceNotFound(device_name.to_string()))?;

        let (stream_config, sample_format) =
            get_low_latency_config(&device, Some(config.sample_rate), Some(config.buffer_size))?;

        let processor = self.processor.clone();
        let callback: AudioCallback = Box::new(move |input, output| {
            if let Some(ref mut proc) = *processor.lock() {
                proc.process(input, output);
            } else {
                for (out, inp) in output.iter_mut().zip(input.iter()) {
                    *out = *inp;
                }
            }
        });

        let stream = AudioStream::new_output(&device, &stream_config, sample_format, callback)?;
        stream.start()?;

        *self.active_stream.lock() = Some(stream);
        *self.current_device.lock() = Some(device_info.clone());
        self.current_config = config;

        info!("Audio engine started successfully");
        Ok(())
    }

    pub fn start(&mut self) -> AudioResult<()> {
        let default_device = self.device_manager.get_default_output()?;
        self.start_with_device(&default_device.name, EngineConfig::default())
    }

    pub fn stop(&self) -> AudioResult<()> {
        if let Some(stream) = self.active_stream.lock().take() {
            stream.stop()?;
        }
        info!("Audio engine stopped");
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.active_stream
            .lock()
            .as_ref()
            .map(|s| s.is_running())
            .unwrap_or(false)
    }

    pub fn current_latency_ms(&self) -> Option<f32> {
        self.active_stream.lock().as_ref().map(|s| s.latency_ms())
    }

    pub fn current_device(&self) -> Option<AudioDeviceInfo> {
        self.current_device.lock().clone()
    }

    pub fn current_config(&self) -> &EngineConfig {
        &self.current_config
    }

    pub fn target_latency_ms(&self) -> f32 {
        TARGET_LATENCY_MS
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
