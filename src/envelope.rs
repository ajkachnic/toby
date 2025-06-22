pub struct ADSR {
    /// The attack time in seconds
    pub attack: f32,
    /// The decay time in seconds
    pub decay: f32,
    /// The sustain level in the range [0, 1]
    pub sustain: f32,
    /// The release time in seconds
    pub release: f32,

    pub stage: EnvelopeStage,
    pub timer: f32,
}

impl Default for ADSR {
    fn default() -> Self {
        Self {
            attack: 0.5,
            decay: 0.001,
            sustain: 0.8,
            release: 0.1,

            stage: EnvelopeStage::Attack,
            timer: 0.0,
        }
    }
}

impl ADSR {
    pub fn reset(&mut self) {
        self.stage = EnvelopeStage::Release;
        self.timer = self.release;
    }

    pub fn trigger(&mut self, event: EnvelopeEvent) {
        self.timer = 0.0;
        match event {
            EnvelopeEvent::Attack => {
                self.stage = EnvelopeStage::Attack;
            }
            EnvelopeEvent::Release => {
                self.stage = EnvelopeStage::Release;
            }
        }
    }

    pub fn next(&mut self, sample_rate: f32) -> f32 {
        match self.stage {
            EnvelopeStage::Attack => {
                if self.timer >= self.attack {
                    self.stage = EnvelopeStage::Decay;
                    self.timer = 0.0;

                    return 1.0;
                }

                self.timer += 1.0 / sample_rate;

                let x = interpolate(self.timer / self.attack, 0.0, 1.0);

                return x;
            }
            EnvelopeStage::Decay => {
                if self.timer >= self.decay {
                    self.stage = EnvelopeStage::Sustain;
                    self.timer = 0.0;

                    return self.sustain;
                }

                self.timer += 1.0 / sample_rate;

                // Linear step from
                let x = interpolate(self.timer / self.decay, 1.0, self.sustain);

                return x;
            }
            EnvelopeStage::Sustain => self.sustain,
            EnvelopeStage::Release => {
                if self.timer >= self.release {
                    return 0.0;
                }

                self.timer += 1.0 / sample_rate;

                let x = interpolate(self.timer / self.release, self.sustain, 0.0);
                return x;
            }
        }
    }
}

pub enum EnvelopeEvent {
    Attack,
    Release,
}

#[derive(Clone, Copy, PartialEq)]
pub enum EnvelopeStage {
    Attack,
    Decay,
    Sustain,
    Release,
}

/// Linear interpolation between two values
///
/// Value should be between [0, 1]
fn interpolate(value: f32, from: f32, to: f32) -> f32 {
    return from * (1.0 - value) + to * value;
}
