use nih_plug::prelude::{Smoother, SmoothingStyle};

use crate::oscillator;

const MAX_FREQUENCY: f32 = 0.25;
const MIN_FREQUENCY: f32 = 0.000001;

pub struct SuperSquareOscillator {
    // state
    master_phase: f32,
    slave_phase: f32,
    next_sample: f32,
    high: bool,

    // params
    master_frequency: Smoother<f32>,
    slave_frequency: Smoother<f32>,
}

impl SuperSquareOscillator {
    pub fn prepare(&mut self, shape: f32, frequency: f32, sample_rate: f32) {
        let master_frequency = frequency;
        let slave_frequency = if shape < 0.5 {
            frequency * (0.51 + 0.98 * shape)
        } else {
            frequency * (1.0 + 16.0 * (shape - 0.5) * (shape - 0.5))
        };

        self.master_frequency
            .set_target(sample_rate, frequency.clamp(MIN_FREQUENCY, MAX_FREQUENCY));
        _ = self.master_frequency.next();
        self.slave_frequency.set_target(
            sample_rate,
            slave_frequency.clamp(MIN_FREQUENCY, MAX_FREQUENCY),
        );
        _ = self.slave_frequency.next();
    }

    pub fn process(&mut self, frequency: f32, sample_rate: f32) -> f32 {
        let mut reset = false;
        let mut transition_during_reset = false;
        let mut reset_time = 0.0;

        let mut this_sample = self.next_sample;
        self.next_sample = 0.0;

        let master_frequency = self.master_frequency.next();
        let slave_frequency = self.slave_frequency.next();

        self.master_phase += master_frequency;
        if self.master_phase >= 1.0 {
            self.master_phase -= 1.0;
            reset_time = self.master_phase / master_frequency;

            let mut slave_phase_at_reset = self.slave_phase + (1.0 - reset_time) * slave_frequency;
            reset = true;

            if slave_phase_at_reset >= 1.0 {
                slave_phase_at_reset -= 1.0;
                transition_during_reset = true;
            }
            if !self.high && slave_phase_at_reset >= 0.5 {
                transition_during_reset = true;
            }

            let value = if slave_phase_at_reset < 0.5 { 0.0 } else { 1.0 };

            this_sample -= value * oscillator::this_blep_sample(reset_time);
            self.next_sample -= value * oscillator::next_blep_sample(reset_time);
        }

        self.slave_phase += slave_frequency;
        while transition_during_reset || !reset {
            if !self.high {
                if self.slave_phase < 0.5 {
                    break;
                }
                let t = (self.slave_phase - 0.5) / slave_frequency;
                this_sample += oscillator::this_blep_sample(t);
                self.next_sample += oscillator::next_blep_sample(t);
                self.high = true;
            }

            if self.high {
                if self.slave_phase < 1.0 {
                    break;
                }

                self.slave_phase -= 1.0;
                let t = self.slave_phase / slave_frequency;
                this_sample -= oscillator::this_blep_sample(t);
                self.next_sample -= oscillator::next_blep_sample(t);
                self.high = false;
            }
        }

        if reset {
            self.slave_phase = reset_time * slave_frequency;
            self.high = false;
        }

        self.next_sample += if self.slave_phase < 0.5 { 0.0 } else { 1.0 };

        return 2.0 * this_sample - 1.0;
    }
}

impl Default for SuperSquareOscillator {
    fn default() -> Self {
        Self {
            master_phase: 0.0,
            slave_phase: 0.0,
            next_sample: 0.0,
            high: false,

            master_frequency: Smoother::new(SmoothingStyle::Linear(2.0)),
            slave_frequency: Smoother::new(SmoothingStyle::Linear(2.0)),
        }
    }
}
