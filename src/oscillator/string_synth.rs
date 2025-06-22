use nih_plug::prelude::{Smoother, SmoothingStyle};

use crate::oscillator;

const MAX_FREQUENCY: f32 = 0.25;
const MIN_FREQUENCY: f32 = 0.000001;

pub struct StringSynthOscillator {
    // state
    phase: f32,
    segment: i32,
    next_sample: f32,

    // params
    frequency: Smoother<f32>,
    saw_8_gain: Smoother<f32>,
    saw_4_gain: Smoother<f32>,
    saw_2_gain: Smoother<f32>,
    saw_1_gain: Smoother<f32>,
}

impl StringSynthOscillator {
    pub fn prepare_block(
        &mut self,
        unshifted_registration: &[f32],
        gain: f32,
        frequency: f32,
        sample_rate: f32,
    ) {
        // Deal with very high frequencies by shifting everything 1 or 2 octave
        // down: Instead of playing the 1st harmonic of a 8kHz wave, we play the
        // second harmonic of a 4kHz wave.
        let mut phase_delta = frequency / sample_rate;
        let mut shift = 0;
        while phase_delta > 0.5 {
            shift += 2;
            phase_delta *= 0.5;
        }

        // frequency is too high, return
        if shift >= 8 {
            return;
        }

        let mut registration = [0.0; 7];
        registration[shift..7].copy_from_slice(&unshifted_registration[0..(7 - shift)]);

        self.frequency
            .set_target(sample_rate, phase_delta.clamp(MIN_FREQUENCY, MAX_FREQUENCY));
        _ = self.frequency.next();
        self.saw_8_gain.set_target(
            sample_rate,
            gain * (registration[0] + 2.0 * registration[1]),
        );
        _ = self.saw_8_gain.next();
        self.saw_4_gain.set_target(
            sample_rate,
            gain * (registration[2] - registration[1] + 2.0 * registration[3]),
        );
        _ = self.saw_4_gain.next();
        self.saw_2_gain.set_target(
            sample_rate,
            gain * (registration[4] - registration[3] + 2.0 * registration[5]),
        );
        _ = self.saw_2_gain.next();
        self.saw_1_gain
            .set_target(sample_rate, gain * (registration[6] - registration[5]));
        _ = self.saw_1_gain.next();
    }

    pub fn process(&mut self) -> f32 {
        let mut this_sample = self.next_sample;
        self.next_sample = 0.0;

        let frequency = self.frequency.next();
        let saw_8_gain = self.saw_8_gain.next();
        let saw_4_gain = self.saw_4_gain.next();
        let saw_2_gain = self.saw_2_gain.next();
        let saw_1_gain = self.saw_1_gain.next();

        self.phase += frequency;

        let mut next_segment = self.phase as i32;
        if next_segment != self.segment {
            let mut discontinuity = 0.0;
            if next_segment == 8 {
                self.phase -= 8.0;
                next_segment -= 8;
                discontinuity -= saw_8_gain;
            }

            if (next_segment & 3) == 0 {
                discontinuity -= saw_4_gain;
            }

            if (next_segment & 1) == 0 {
                discontinuity -= saw_2_gain;
            }

            discontinuity -= saw_1_gain;

            if discontinuity != 0.0 {
                let fraction = self.phase - next_segment as f32;
                let t = fraction / frequency;
                this_sample += discontinuity * oscillator::this_blep_sample(t);
                self.next_sample = discontinuity * oscillator::next_blep_sample(t);
            }
        }
        self.segment = next_segment;

        self.next_sample += (self.phase - 4.0) * saw_8_gain * 0.125;
        self.next_sample += (self.phase - (self.segment & 4) as f32 - 2.0) * saw_4_gain * 0.25;
        self.next_sample += (self.phase - (self.segment & 6) as f32 - 1.0) * saw_2_gain * 0.5;
        self.next_sample += (self.phase - (self.segment & 7) as f32 - 0.0) * saw_1_gain;

        2.0 * this_sample
    }
}

impl Default for StringSynthOscillator {
    fn default() -> Self {
        Self {
            phase: 0.0,
            segment: 0,
            next_sample: 0.0,

            frequency: Smoother::new(SmoothingStyle::Linear(4.0)),
            saw_8_gain: Smoother::new(SmoothingStyle::Linear(4.0)),
            saw_4_gain: Smoother::new(SmoothingStyle::Linear(4.0)),
            saw_2_gain: Smoother::new(SmoothingStyle::Linear(4.0)),
            saw_1_gain: Smoother::new(SmoothingStyle::Linear(4.0)),
        }
    }
}

pub const REGISTRATION_TABLE: [[f32; 7]; 11] = [
    [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], // Saw
    [0.5, 0.0, 0.5, 0.0, 0.0, 0.0, 0.0], // Saw + saw
    [0.4, 0.0, 0.2, 0.0, 0.4, 0.0, 0.0], // Full saw
    [0.3, 0.0, 0.0, 0.3, 0.0, 0.4, 0.0], // Full saw + square hybrid
    [0.3, 0.0, 0.0, 0.0, 0.0, 0.7, 0.0], // Saw + high square harmo
    [0.2, 0.0, 0.0, 0.2, 0.0, 0.6, 0.0], // Weird hybrid
    [0.0, 0.2, 0.1, 0.0, 0.2, 0.5, 0.0], // Sawsquare high harmo
    [0.0, 0.3, 0.0, 0.3, 0.0, 0.4, 0.0], // Square high armo
    [0.0, 0.4, 0.0, 0.3, 0.0, 0.3, 0.0], // Full square
    [0.0, 0.5, 0.0, 0.5, 0.0, 0.0, 0.0], // Square + Square
    [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0], // Square
];
