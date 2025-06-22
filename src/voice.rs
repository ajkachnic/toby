use nih_plug::util;

use crate::{
    envelope::{self, EnvelopeEvent, EnvelopeStage},
    filter,
    oscillator::engine::{OscillatorEngine, OscillatorParams, OscillatorType},
};

pub struct Voice {
    pub oscillator: OscillatorEngine,
    pub filter: filter::Svf,
    pub envelope: envelope::ADSR,

    pub midi_note_id: u8,
    pub midi_note_freq: f32,
    pub midi_velocity: f32,

    pub trigger_time: f32,
}

impl Voice {
    pub fn new() -> Self {
        Self {
            oscillator: OscillatorEngine::default(),
            filter: filter::Svf::default(),
            envelope: envelope::ADSR::default(),

            midi_note_id: 0,
            midi_note_freq: 1.0,
            midi_velocity: 1.0,

            trigger_time: 0.0,
        }
    }

    pub fn is_active(&self) -> bool {
        self.envelope.is_active()
    }

    pub fn trigger(&mut self, note: u8, velocity: f32) {
        self.midi_note_id = note;
        self.midi_note_freq = util::midi_note_to_freq(note);
        self.midi_velocity = velocity;

        self.envelope.trigger(EnvelopeEvent::Attack);
    }

    pub fn release(&mut self) {
        self.envelope.trigger(EnvelopeEvent::Release);
    }

    pub fn prepare_block(&mut self, params: VoiceParams, sample_rate: f32) {
        self.oscillator.selected = params.oscillator_type;
        self.oscillator.prepare_block(
            OscillatorParams {
                shape: params.shape,
                morph: params.morph,
            },
            self.midi_note_freq,
            sample_rate,
        );
    }

    pub fn process(&mut self, params: VoiceParams, sample_rate: f32) -> f32 {
        let gain = params.gain;
        let cutoff = params.cutoff;
        let resonance = params.resonance;

        self.filter.set_f_q(cutoff / sample_rate, resonance);

        let v = self.oscillator.process(self.midi_note_freq, sample_rate);
        let v = v * self.envelope.next(sample_rate);
        let v = self.filter.process(v);

        v * util::db_to_gain_fast(gain)
    }
}

#[derive(Clone, Copy)]
pub struct VoiceParams {
    pub oscillator_type: OscillatorType,
    pub shape: f32,
    pub morph: f32,

    pub gain: f32,
    pub cutoff: f32,
    pub resonance: f32,
}
