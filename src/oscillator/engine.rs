use nih_plug::prelude::Enum;

use crate::oscillator::{SuperSquareOscillator, VariableSawOscillator};

#[derive(Enum, PartialEq, Debug, Clone, Copy)]
pub enum OscillatorType {
    SuperSquare,
    VariableSaw,
}

#[derive(Clone, Copy)]
pub struct OscillatorParams {
    pub shape: f32,
    pub morph: f32,
}

pub struct OscillatorEngine {
    super_square: SuperSquareOscillator,
    variable_saw: VariableSawOscillator,
    pub selected: OscillatorType,
}

impl OscillatorEngine {
    pub fn new() -> Self {
        Self {
            super_square: SuperSquareOscillator::default(),
            variable_saw: VariableSawOscillator::default(),
            selected: OscillatorType::SuperSquare,
        }
    }

    pub fn prepare_block(&mut self, params: OscillatorParams, frequency: f32, sample_rate: f32) {
        match self.selected {
            OscillatorType::SuperSquare => {
                self.super_square
                    .prepare_block(params.shape, frequency, sample_rate);
            }
            OscillatorType::VariableSaw => {
                let saw_pw = if params.morph < 0.5 {
                    params.morph + 0.5
                } else {
                    1.0 - (params.morph - 0.5) * 2.0
                };

                let saw_pw = (saw_pw * 1.1).clamp(0.005, 1.0);
                let saw_shape = (10.0 - 21.0 * params.shape).clamp(0.0, 1.0);
                self.variable_saw
                    .prepare_block(saw_pw, saw_shape, frequency, sample_rate);
            }
        }
    }

    pub fn process(&mut self, frequency: f32, sample_rate: f32) -> f32 {
        match self.selected {
            OscillatorType::SuperSquare => self.super_square.process(),
            OscillatorType::VariableSaw => self.variable_saw.process(frequency, sample_rate),
        }
    }
}

impl Default for OscillatorEngine {
    fn default() -> Self {
        Self::new()
    }
}
