#[derive(Debug, Clone, Copy, PartialEq)]
enum EnvelopeState {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// Simple ADSR envelope generator.
pub struct Envelope {
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    state: EnvelopeState,
    level: f32,
    sample_rate: f32,
}

impl Envelope {
    pub fn new() -> Self {
        Self::with_sample_rate(48_000.0)
    }

    pub fn with_sample_rate(sample_rate: f32) -> Self {
        Self {
            attack: 0.01,
            decay: 0.1,
            sustain: 0.7,
            release: 0.3,
            state: EnvelopeState::Idle,
            level: 0.0,
            sample_rate,
        }
    }

    pub fn set_adsr(&mut self, attack: f32, decay: f32, sustain: f32, release: f32) {
        self.attack = attack.max(f32::EPSILON);
        self.decay = decay.max(f32::EPSILON);
        self.sustain = sustain.clamp(0.0, 1.0);
        self.release = release.max(f32::EPSILON);
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
    }

    pub fn trigger(&mut self) {
        self.state = EnvelopeState::Attack;
    }

    pub fn release(&mut self) {
        if self.state != EnvelopeState::Idle {
            self.state = EnvelopeState::Release;
        }
    }

    pub fn reset(&mut self) {
        self.state = EnvelopeState::Idle;
        self.level = 0.0;
    }

    pub fn is_active(&self) -> bool {
        self.state != EnvelopeState::Idle
    }

    pub fn next_sample(&mut self) -> f32 {
        let attack_inc = 1.0 / (self.attack * self.sample_rate);
        let decay_inc = (1.0 - self.sustain) / (self.decay * self.sample_rate);
        let release_inc = self.level / (self.release * self.sample_rate);

        match self.state {
            EnvelopeState::Idle => {
                self.level = 0.0;
            }
            EnvelopeState::Attack => {
                self.level += attack_inc;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.state = EnvelopeState::Decay;
                }
            }
            EnvelopeState::Decay => {
                self.level -= decay_inc;
                if self.level <= self.sustain {
                    self.level = self.sustain;
                    self.state = EnvelopeState::Sustain;
                }
            }
            EnvelopeState::Sustain => {
                self.level = self.sustain;
            }
            EnvelopeState::Release => {
                self.level -= release_inc;
                if self.level <= 0.0 {
                    self.level = 0.0;
                    self.state = EnvelopeState::Idle;
                }
            }
        }

        self.level
    }
}
