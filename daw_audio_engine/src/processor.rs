pub struct ProcessorConfig {
    pub sample_rate: f32,
    pub buffer_size: usize,
    pub input_channels: usize,
    pub output_channels: usize,
}

pub trait AudioProcessor: Send {
    fn process(&mut self, input: &[f32], output: &mut [f32]);
    fn configure(&mut self, config: &ProcessorConfig);
    fn name(&self) -> &str;
}

pub struct PassThroughProcessor;

impl PassThroughProcessor {
    pub fn new() -> Self {
        Self
    }
}

impl AudioProcessor for PassThroughProcessor {
    fn process(&mut self, input: &[f32], output: &mut [f32]) {
        for (out, inp) in output.iter_mut().zip(input.iter()) {
            *out = *inp;
        }
    }

    fn configure(&mut self, _config: &ProcessorConfig) {}

    fn name(&self) -> &str {
        "PassThrough"
    }
}

pub struct GainProcessor {
    gain: f32,
    config: Option<ProcessorConfig>,
}

impl GainProcessor {
    pub fn new(gain_db: f32) -> Self {
        let gain = 10.0f32.powf(gain_db / 20.0);
        Self { gain, config: None }
    }

    pub fn set_gain_db(&mut self, gain_db: f32) {
        self.gain = 10.0f32.powf(gain_db / 20.0);
    }
}

impl AudioProcessor for GainProcessor {
    fn process(&mut self, input: &[f32], output: &mut [f32]) {
        for (out, inp) in output.iter_mut().zip(input.iter()) {
            *out = inp * self.gain;
        }
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
        "Gain"
    }
}
