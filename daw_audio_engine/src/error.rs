use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Device error: {0}")]
    Device(String),

    #[error("Stream error: {0}")]
    Stream(String),

    #[error("CPAL error: {0}")]
    Cpal(#[from] cpal::BuildStreamError),

    #[error("Play stream error: {0}")]
    PlayStream(#[from] cpal::PlayStreamError),

    #[error("Pause stream error: {0}")]
    PauseStream(#[from] cpal::PauseStreamError),

    #[error("Devices error: {0}")]
    Devices(String),

    #[error("Default stream config error: {0}")]
    DefaultStreamConfig(String),

    #[error("Supported stream configs error: {0}")]
    SupportedStreamConfigs(String),

    #[error("Device name error: {0}")]
    DeviceName(String),

    #[error("No default device found")]
    NoDefaultDevice,

    #[error("Device name not found: {0}")]
    DeviceNotFound(String),

    #[error("Engine not running")]
    EngineNotRunning,

    #[error("Invalid sample rate: {0}")]
    InvalidSampleRate(u32),

    #[error("Invalid buffer size: {0}")]
    InvalidBufferSize(u32),

    #[error("ASIO not available")]
    AsioNotAvailable,

    #[error("Other: {0}")]
    Other(String),
}

pub type AudioResult<T> = Result<T, AudioError>;

impl From<cpal::DevicesError> for AudioError {
    fn from(err: cpal::DevicesError) -> Self {
        AudioError::Devices(err.to_string())
    }
}

impl From<cpal::DeviceNameError> for AudioError {
    fn from(err: cpal::DeviceNameError) -> Self {
        AudioError::DeviceName(err.to_string())
    }
}

impl From<cpal::DefaultStreamConfigError> for AudioError {
    fn from(err: cpal::DefaultStreamConfigError) -> Self {
        AudioError::DefaultStreamConfig(err.to_string())
    }
}

impl From<cpal::SupportedStreamConfigsError> for AudioError {
    fn from(err: cpal::SupportedStreamConfigsError) -> Self {
        AudioError::SupportedStreamConfigs(err.to_string())
    }
}
