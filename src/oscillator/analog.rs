use super::{
    next_blep_sample, next_integrated_blep_sample, this_blep_sample, this_integrated_blep_sample,
};

const NOTCH_DEPTH: f32 = 0.2;

struct VariableShapeOscillator {
    // Parameters
    pw: f32,
    waveshape: f32,
    master_frequency: f32,
    slave_frequency: f32,
    phase_modulation: f32,

    // State
    master_phase: f32,
    slave_phase: f32,
    next_sample: f32,
    previous_pw: f32,
    high: bool,
}

impl Default for VariableShapeOscillator {
    fn default() -> Self {
        Self {
            master_phase: 0.0,
            slave_phase: 0.0,
            next_sample: 0.0,
            previous_pw: 0.5,
            high: false,

            pw: 0.5,
            waveshape: 0.0,
            master_frequency: 0.0,
            slave_frequency: 0.0,
            phase_modulation: 0.0,
        }
    }
}

const ENABLE_SYNC: bool = true;
const OUTPUT_PHASE: bool = true;

impl VariableShapeOscillator {
    pub fn prepare(&mut self, pw: f32, waveshape: f32, frequency: f32, sample_rate: f32) {
        let phase_delta = frequency / sample_rate;
        let pw = if phase_delta >= 0.25 {
            0.5
        } else {
            pw.clamp(phase_delta * 2.0, 1.0 - 2.0 * phase_delta)
        };

        self.pw = pw;
        self.waveshape = waveshape;
    }

    pub fn process(
        &mut self,
        master_frequency: f32,
        slave_frequency: f32,
        sample_rate: f32,
    ) -> f32 {
        let mut this_sample = self.next_sample;
        self.next_sample = 0.0;

        let mut reset = false;
        let mut transition_during_reset = false;
        let mut reset_time = 0.0;

        let master_phase_delta = master_frequency / sample_rate;
        let slave_phase_delta = slave_frequency / sample_rate;
        let square_amount = (self.waveshape - 0.5).max(0.0) * 2.0;
        let triangle_amount = (1.0 - self.waveshape - 2.0).max(0.0);

        let slope_up = 1.0 / self.pw;
        let slope_down = 1.0 / (1.0 - self.pw);

        if ENABLE_SYNC {
            self.master_phase += master_phase_delta;
            if self.master_phase >= 1.0 {
                self.master_phase -= 1.0;
                reset_time = self.master_phase / master_phase_delta;

                let mut slave_phase_at_reset =
                    self.slave_phase + (1.0 - reset_time) * slave_phase_delta;
                reset = true;

                if slave_phase_at_reset >= 1.0 {
                    slave_phase_at_reset -= 1.0;
                    transition_during_reset = true;
                }

                if !self.high && slave_phase_at_reset >= self.pw {
                    transition_during_reset = true;
                }

                let value = self.compute_naive_sample(
                    slave_phase_at_reset,
                    self.pw,
                    slope_up,
                    slope_down,
                    triangle_amount,
                    square_amount,
                );

                this_sample -= value * this_blep_sample(reset_time);
                self.next_sample -= value * next_blep_sample(reset_time);
            }
        } else if OUTPUT_PHASE {
            self.master_phase += master_phase_delta;
            if self.master_phase >= 1.0 {
                self.master_phase -= 1.0;
            }
        }

        self.slave_phase += slave_phase_delta;
        while transition_during_reset || !reset {
            if !self.high {
                if self.slave_phase < self.pw {
                    break;
                }

                let t =
                    (self.slave_phase - self.pw) / (self.previous_pw - self.pw + slave_phase_delta);
                let triangle_step = (slope_up + slope_down) * slave_phase_delta * triangle_amount;

                this_sample += square_amount * this_blep_sample(t);
                self.next_sample += square_amount * next_blep_sample(t);

                this_sample -= triangle_step * this_integrated_blep_sample(t);
                self.next_sample -= triangle_step * next_integrated_blep_sample(t);

                self.high = true;
            }

            if self.high {
                if self.slave_phase < 1.0 {
                    break;
                }
                self.slave_phase -= 1.0;

                let t = self.slave_phase / slave_phase_delta;
                let triangle_step = (slope_up + slope_down) * slave_phase_delta * triangle_amount;

                this_sample -= (1.0 - triangle_amount) * this_blep_sample(t);
                self.next_sample -= (1.0 - triangle_amount) * next_blep_sample(t);

                this_sample += triangle_step * this_integrated_blep_sample(t);
                self.next_sample += triangle_step * next_integrated_blep_sample(t);

                self.high = false;
            }
        }

        if ENABLE_SYNC && reset {
            self.slave_phase = reset_time * slave_phase_delta;
            self.high = false;
        }

        self.next_sample += self.compute_naive_sample(
            self.slave_phase,
            self.pw,
            slope_up,
            slope_down,
            triangle_amount,
            square_amount,
        );
        self.previous_pw = self.pw;

        if OUTPUT_PHASE {
            let mut phasor = self.master_phase;
            if ENABLE_SYNC {
                // A trick to prevent discontinuities when the phase wraps around.
                let w = 4.0 * (1.0 - self.master_phase) * self.master_phase;
                this_sample *= w * (2.0 - w);

                // Apply some asymmetry on the main phasor too.
                let p2 = phasor * phasor;
                phasor += (p2 * p2 - phasor) * (self.pw - 0.5).abs() * 2.0;
            }
            return phasor + self.phase_modulation * this_sample;
        } else {
            return 2.0 * this_sample - 1.0;
        }
    }

    fn compute_naive_sample(
        &self,
        phase: f32,
        pw: f32,
        slope_up: f32,
        slope_down: f32,
        triangle_amount: f32,
        square_amount: f32,
    ) -> f32 {
        let mut saw = phase;
        let square = if phase < pw { 0.0 } else { 1.0 };
        let triangle = if phase < pw {
            phase * slope_up
        } else {
            1.0 - (phase - pw) * slope_down
        };

        saw += (square - saw) * square_amount;
        saw += (triangle - saw) * triangle_amount;
        return saw;
    }
}
