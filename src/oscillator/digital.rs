use std::f32::consts;

pub enum Shape {
    Saw,
    Square,
    Triangle,
    Sine,
}

/// Simple digital oscillator implementation. Digital in the sense that these oscillators
/// produce a pure value for their functions. There's no distortion or warping or wave-shaping.
///
/// The "analog" oscillators have more imperfections in their sound, matching how an analog VCO
/// wouldn't produce a perfect or pure wave.
#[allow(dead_code)]
pub struct DigitalOscillator {
    phase: f32,
    pub(crate) shape: Shape,
}

impl Default for DigitalOscillator {
    fn default() -> Self {
        Self {
            phase: 0.0,
            shape: Shape::Sine,
        }
    }
}

impl DigitalOscillator {
    pub fn new(shape: Shape) -> Self {
        Self { phase: 0.0, shape }
    }

    fn process_sine(&mut self, frequency: f32, sample_rate: f32) -> f32 {
        let phase_delta = frequency / sample_rate;

        // Multiply by tau to make the period = 1
        let sine = (self.phase * consts::TAU).sin();

        self.phase += phase_delta;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        sine
    }

    fn process_square(&mut self, frequency: f32, sample_rate: f32) -> f32 {
        let phase_delta = frequency / sample_rate;

        let square = if self.phase >= 0.5 { 0.0 } else { 1.0 };

        self.phase += phase_delta;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        square
    }

    pub fn process(&mut self, frequency: f32, sample_rate: f32) -> f32 {
        match self.shape {
            Shape::Sine => self.process_sine(frequency, sample_rate),
            Shape::Square => self.process_square(frequency, sample_rate),
            _ => todo!(),
        }
    }
}

pub struct BlendOscillator {
    pub shape: f32,
    a: DigitalOscillator,
    b: DigitalOscillator,
}

impl Default for BlendOscillator {
    fn default() -> Self {
        Self {
            shape: 0.5,
            a: DigitalOscillator::new(Shape::Sine),
            b: DigitalOscillator::new(Shape::Square),
        }
    }
}

impl BlendOscillator {
    pub fn process(&mut self, frequency: f32, sample_rate: f32) -> f32 {
        let a = self.a.process(frequency, sample_rate);
        let b = self.b.process(frequency, sample_rate);

        return (a * (1.0 - self.shape)) + (b * self.shape);
    }
}
