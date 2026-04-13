use crate::error::{AudioError, AudioResult};
use crate::hardware::{scan_hardware_devices, HardwareDevice};
use cpal::{
    traits::{DeviceTrait, HostTrait},
    Device, Host, SampleFormat,
};
use std::fmt;

pub struct DeviceManager {
    host: Host,
}

impl DeviceManager {
    pub fn new() -> Self {
        Self {
            host: cpal::default_host(),
        }
    }

    pub fn with_host(host: Host) -> Self {
        Self { host }
    }

    pub fn enumerate_devices(&self) -> AudioResult<Vec<AudioDeviceInfo>> {
        let mut devices = Vec::new();

        for device in self.host.output_devices()? {
            if let Ok(info) = Self::get_device_info(&device) {
                devices.push(info);
            }
        }

        for device in self.host.input_devices()? {
            if let Ok(info) = Self::get_device_info(&device) {
                if !devices
                    .iter()
                    .any(|d: &AudioDeviceInfo| d.name == info.name)
                {
                    devices.push(info);
                }
            }
        }

        Ok(devices)
    }

    pub fn get_default_output(&self) -> AudioResult<AudioDeviceInfo> {
        let device = self
            .host
            .default_output_device()
            .ok_or(AudioError::NoDefaultDevice)?;
        Self::get_device_info(&device)
    }

    pub fn get_default_input(&self) -> AudioResult<AudioDeviceInfo> {
        let device = self
            .host
            .default_input_device()
            .ok_or(AudioError::NoDefaultDevice)?;
        Self::get_device_info(&device)
    }

    pub fn find_device_by_name(&self, name: &str) -> AudioResult<AudioDeviceInfo> {
        let devices = self.enumerate_devices()?;
        devices
            .into_iter()
            .find(|d| d.name == name)
            .ok_or_else(|| AudioError::DeviceNotFound(name.to_string()))
    }

    /// Scan for raw hardware devices (bypassing ALSA plugins)
    pub fn enumerate_hardware_devices(&self) -> Vec<HardwareDevice> {
        scan_hardware_devices()
    }

    /// Find Focusrite/Scarlett devices specifically
    pub fn find_focusrite_devices(&self) -> Vec<HardwareDevice> {
        scan_hardware_devices()
            .into_iter()
            .filter(|d| Self::is_asio_device(&d.card_name))
            .collect()
    }

    fn get_device_info(device: &Device) -> AudioResult<AudioDeviceInfo> {
        let name = device.name()?;
        let is_output = device.default_output_config().is_ok();
        let is_input = device.default_input_config().is_ok();

        let default_config = if is_output {
            device.default_output_config()?
        } else if is_input {
            device.default_input_config()?
        } else {
            return Err(AudioError::Device(
                "Device has no supported configs".to_string(),
            ));
        };

        let mut sample_rates = Vec::new();
        let mut buffer_sizes = Vec::new();

        if is_output {
            for config_range in device.supported_output_configs()? {
                sample_rates.push(config_range.min_sample_rate().0);
                sample_rates.push(config_range.max_sample_rate().0);

                // Handle buffer sizes - cpal provides range or fixed values
                match config_range.buffer_size() {
                    cpal::SupportedBufferSize::Range { min, max } => {
                        buffer_sizes.push(*min);
                        buffer_sizes.push(*max);
                    }
                    cpal::SupportedBufferSize::Unknown => {}
                }
            }
        }

        sample_rates.sort();
        sample_rates.dedup();
        buffer_sizes.sort();
        buffer_sizes.dedup();

        let is_asio = Self::is_asio_device(&name);

        Ok(AudioDeviceInfo {
            name,
            is_output,
            is_input,
            sample_rate: default_config.sample_rate().0,
            sample_format: default_config.sample_format(),
            channels: default_config.channels(),
            available_sample_rates: sample_rates,
            available_buffer_sizes: buffer_sizes,
            is_asio,
        })
    }

    fn is_asio_device(name: &str) -> bool {
        let name_lower = name.to_lowercase();
        name_lower.contains("asio")
            || name_lower.contains("focusrite")
            || name_lower.contains("scarlett")
            || name_lower.contains("universal audio")
            || name_lower.contains("rme")
            || name_lower.contains("motu")
            || name_lower.contains("steinberg")
    }
}

#[derive(Debug, Clone)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub is_output: bool,
    pub is_input: bool,
    pub sample_rate: u32,
    pub sample_format: SampleFormat,
    pub channels: u16,
    pub available_sample_rates: Vec<u32>,
    pub available_buffer_sizes: Vec<u32>,
    pub is_asio: bool,
}

impl fmt::Display for AudioDeviceInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let asio_marker = if self.is_asio { " [ASIO]" } else { "" };
        write!(
            f,
            "{}{} - {}Hz, {}ch",
            self.name, asio_marker, self.sample_rate, self.channels
        )
    }
}
