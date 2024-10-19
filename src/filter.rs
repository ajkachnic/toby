use std::f32::consts;

pub enum FilterMode {
    LowPass,
    BandPass,
    HighPass,
}
pub struct Svf {
    mode: FilterMode,

    g: f32,
    r: f32,
    h: f32,

    state_1: f32,
    state_2: f32,
}

impl Default for Svf {
    fn default() -> Self {
        let mut this = Self {
            mode: FilterMode::LowPass,

            g: 0.0,
            r: 0.0,
            h: 0.0,
            state_1: 0.0,
            state_2: 0.0,
        };

        this.set_f_q(22_000.0, 1.0);

        this
    }
}

impl Svf {
    /// Set frequency and resonance from true units.
    pub fn set_f_q(&mut self, f: f32, resonance: f32) {
        self.g = tan(f);
        self.r = 1.0 / resonance;
        self.h = 1.0 / (1.0 + self.r * self.g + self.g * self.g);
    }

    pub fn process(&mut self, i: f32) -> f32 {
        let hp = (i - self.r * self.state_1 - self.g * self.state_1 - self.state_2) * self.h;
        let bp = self.g * hp + self.state_1;
        let lp = self.g * bp + self.state_2;

        self.state_1 = self.g * hp + bp;
        self.state_2 = self.g * bp + lp;

        match self.mode {
            FilterMode::LowPass => lp,
            FilterMode::BandPass => bp,
            FilterMode::HighPass => hp,
        }
    }
}

fn tan(x: f32) -> f32 {
    let f = if x < 0.497 { x } else { 0.497 };
    (f * consts::PI).tan()
}
