use std::sync::Arc;

pub struct AudioBuffer {
    data: Vec<f32>,
    channels: usize,
    frames: usize,
}

impl AudioBuffer {
    pub fn new(channels: usize, frames: usize) -> Self {
        Self {
            data: vec![0.0; channels * frames],
            channels,
            frames,
        }
    }

    pub fn from_interleaved(data: Vec<f32>, channels: usize) -> Self {
        let frames = data.len() / channels;
        Self {
            data,
            channels,
            frames,
        }
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn frames(&self) -> usize {
        self.frames
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn sample(&self, channel: usize, frame: usize) -> f32 {
        self.data[frame * self.channels + channel]
    }

    pub fn set_sample(&mut self, channel: usize, frame: usize, value: f32) {
        self.data[frame * self.channels + channel] = value;
    }

    pub fn channel_data(&self, channel: usize) -> &[f32] {
        let start = channel;
        let end = self.data.len();
        &self.data[start..end]
    }

    pub fn channel_data_mut(&mut self, channel: usize) -> &mut [f32] {
        let start = channel;
        let end = self.data.len();
        &mut self.data[start..end]
    }

    pub fn as_interleaved(&self) -> &[f32] {
        &self.data
    }

    pub fn as_interleaved_mut(&mut self) -> &mut [f32] {
        &mut self.data
    }

    pub fn clear(&mut self) {
        self.data.fill(0.0);
    }

    pub fn apply_gain(&mut self, gain: f32) {
        for sample in self.data.iter_mut() {
            *sample *= gain;
        }
    }
}

pub struct OwnedAudioBuffer {
    buffer: Arc<AudioBuffer>,
}

impl OwnedAudioBuffer {
    pub fn new(channels: usize, frames: usize) -> Self {
        Self {
            buffer: Arc::new(AudioBuffer::new(channels, frames)),
        }
    }

    pub fn from_buffer(buffer: AudioBuffer) -> Self {
        Self {
            buffer: Arc::new(buffer),
        }
    }

    pub fn get(&self) -> &AudioBuffer {
        &self.buffer
    }
}

impl Clone for OwnedAudioBuffer {
    fn clone(&self) -> Self {
        Self {
            buffer: Arc::clone(&self.buffer),
        }
    }
}
