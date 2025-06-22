pub mod analog;
pub mod digital;
pub mod engine;
pub mod super_square;

pub use analog::VariableSawOscillator;
pub use super_square::SuperSquareOscillator;

// Ported from Mutable Instruments firmware
pub fn this_blep_sample(t: f32) -> f32 {
    return 0.5 * t * t;
}

pub fn next_blep_sample(t: f32) -> f32 {
    let t = 1.0 - t;

    return 0.5 * t * t;
}

pub fn next_integrated_blep_sample(t: f32) -> f32 {
    let t1 = 0.5 * t;
    let t2 = t1 * t1;
    let t4 = t2 * t2;

    return 0.1875 - t1 + 1.5 * t2 - t4;
}

pub fn this_integrated_blep_sample(t: f32) -> f32 {
    return next_integrated_blep_sample(1.0 - t);
}
