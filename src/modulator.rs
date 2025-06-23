use crate::{resources, util};

pub enum ModulationAlgorithm {
    XFade,
    Fold,
    AnalogRingModulation,
    DigitalRingModulation,
    Xor,
    // Comparator,
    Nop,
}

impl ModulationAlgorithm {
    #[inline]
    pub fn process(&self, modulator: f32, carrier: f32, parameter: f32) -> f32 {
        match self {
            ModulationAlgorithm::XFade => process_x_fade(modulator, carrier, parameter),
            ModulationAlgorithm::Fold => process_fold(modulator, carrier, parameter),
            ModulationAlgorithm::AnalogRingModulation => {
                process_analog_ring_modulation(modulator, carrier, parameter)
            }
            ModulationAlgorithm::DigitalRingModulation => {
                process_digital_ring_modulation(modulator, carrier, parameter)
            }
            ModulationAlgorithm::Xor => process_xor(modulator, carrier, parameter),
            // ModulationAlgorithm::Comparator => process_comparator(modulator, carrier, parameter),
            ModulationAlgorithm::Nop => process_nop(modulator, carrier, parameter),
        }
    }
}

fn process_x_fade(x1: f32, x2: f32, parameter: f32) -> f32 {
    let fade_in = 1.0 - parameter;
    let fade_out = parameter;

    return x1 * fade_in + x2 * fade_out;
}

fn process_fold(x1: f32, x2: f32, parameter: f32) -> f32 {
    let mut sum = 0.0;
    sum += x1;
    sum += x2;
    sum += x1 * x2 * 0.25;

    sum *= 0.02 * parameter;
    let scale = 2048.0 / ((1.0 + 1.0 + 0.25) * 1.02);

    return util::interpolate_table(&resources::LUT_BIPOLAR_FOLD[2048..], sum, scale);
}

fn process_analog_ring_modulation(modulator: f32, carrier: f32, parameter: f32) -> f32 {
    let carrier = carrier * 2.0;
    let ring = diode(modulator + carrier) + diode(modulator - carrier);
    let ring = ring * (4.0 + parameter * 24.0);

    return util::soft_limit(ring);
}

fn process_digital_ring_modulation(modulator: f32, carrier: f32, parameter: f32) -> f32 {
    let ring = 4.0 * modulator * carrier * (1.0 + parameter * 8.0);
    return ring / (1.0 + ring.abs());
}

fn process_xor(x1: f32, x2: f32, parameter: f32) -> f32 {
    let x1_short = util::clip16((x1 * 32768.0) as i32);
    let x2_short = util::clip16((x2 * 32768.0) as i32);

    let modulator = (x1_short ^ x2_short) as f32 / 32768.0;

    let sum = (x1 + x2) * 0.7;

    return sum + (modulator - sum) * parameter;
}

// fn process_comparator(modulator: f32, carrier: f32, parameter: f32) -> f32 {
//     modulator * parameter + carrier * (1.0 - parameter)
// }

fn process_nop(modulator: f32, carrier: f32, parameter: f32) -> f32 {
    modulator
}

fn diode(x: f32) -> f32 {
    // Approximation of diode non-linearity from:
    // Julian Parker - "A simple Digital model of the diode-based ring-modulator."
    // Proc. DAFx-11
    let sign = if x > 0.0 { 1.0 } else { -1.0 };
    let mut dead_zone = x.abs() - 0.667;
    dead_zone += dead_zone.abs();
    dead_zone *= dead_zone;
    return 0.04324765822726063 * dead_zone * sign;
}
