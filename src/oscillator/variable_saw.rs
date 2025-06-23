use super::{
    next_blep_sample, next_integrated_blep_sample, this_blep_sample, this_integrated_blep_sample,
};

const NOTCH_DEPTH: f32 = 0.2;

/// A variable slope saw oscillator with an adjustable notch.
pub struct VariableSawOscillator {
    // Parameters
    pw: f32,
    waveshape: f32,

    // State
    phase: f32,
    high: bool,
    next_sample: f32,
    previous_pw: f32,
}

impl Default for VariableSawOscillator {
    fn default() -> Self {
        Self {
            phase: 0.0,
            next_sample: 0.0,
            previous_pw: 0.5,
            high: false,

            pw: 0.5,
            waveshape: 0.0,
        }
    }
}

impl VariableSawOscillator {
    pub fn prepare_block(&mut self, pw: f32, waveshape: f32, frequency: f32, sample_rate: f32) {
        let phase_delta = frequency / sample_rate;
        let pw = if phase_delta >= 0.25 {
            0.5
        } else {
            pw.clamp(phase_delta * 2.0, 1.0 - 2.0 * phase_delta)
        };

        self.pw = pw;
        self.waveshape = waveshape;
    }

    pub fn process(&mut self, frequency: f32, sample_rate: f32) -> f32 {
        let mut this_sample = self.next_sample;
        self.next_sample = 0.0;

        let phase_delta = frequency / sample_rate;
        let triangle_amount = self.waveshape;
        let notch_amount = 1.0 - self.waveshape;

        let slope_up = 1.0 / self.pw;
        let slope_down = 1.0 / (1.0 - self.pw);

        self.phase += phase_delta;

        if !self.high && self.phase >= self.pw {
            let triangle_step = (slope_up + slope_down) * phase_delta * triangle_amount;
            let notch = (NOTCH_DEPTH + 1.0 - self.pw) * notch_amount;

            let t = (self.phase - self.pw) / (self.previous_pw - self.pw + phase_delta);

            this_sample += notch * this_blep_sample(t);
            self.next_sample += notch * next_blep_sample(t);

            this_sample -= triangle_step * this_integrated_blep_sample(t);
            self.next_sample -= triangle_step * next_integrated_blep_sample(t);

            self.high = true;
        } else if self.phase >= 1.0 {
            self.phase -= 1.0;

            let triangle_step = (slope_up + slope_down) * phase_delta * triangle_amount;
            let notch = (NOTCH_DEPTH + 1.0) * notch_amount;

            let t = self.phase / phase_delta;

            this_sample += notch * this_blep_sample(t);
            self.next_sample += notch * next_blep_sample(t);

            this_sample -= triangle_step * this_integrated_blep_sample(t);
            self.next_sample -= triangle_step * next_integrated_blep_sample(t);

            self.high = false;
        }

        self.next_sample =
            self.compute_naive_sample(slope_up, slope_down, triangle_amount, notch_amount);
        self.previous_pw = self.pw;

        return (2.0 * this_sample - 1.0) / (1.0 + NOTCH_DEPTH);
    }

    fn compute_naive_sample(
        &self,
        slope_up: f32,
        slope_down: f32,
        triangle_amount: f32,
        notch_amount: f32,
    ) -> f32 {
        let notch_saw = if self.phase < self.pw {
            self.phase
        } else {
            1.0 + NOTCH_DEPTH
        };

        let triangle = if self.phase < self.pw {
            self.phase * slope_up
        } else {
            1.0 - (self.phase - self.pw) * slope_down
        };

        return notch_saw * notch_amount + triangle * triangle_amount;
    }
}
