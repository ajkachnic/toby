mod envelope;
mod filter;
mod oscillator;

use envelope::EnvelopeStage;
use nih_plug::prelude::*;
use std::sync::Arc;

use crate::oscillator::engine::{OscillatorEngine, OscillatorParams, OscillatorType};

pub struct Toby {
    params: Arc<TobyParams>,
    sample_rate: f32,

    /// The current phase of the sine wave, always kept between in `[0, 1]`.
    phase: f32,

    /// The MIDI note ID of the active note, if triggered by MIDI.
    midi_note_id: u8,
    /// The frequency if the active note, if triggered by MIDI.
    midi_note_freq: f32,

    oscillator: OscillatorEngine,
    filter: filter::Svf,
    envelope: envelope::ADSR,
}

#[derive(Params)]
struct TobyParams {
    #[id = "gain"]
    pub gain: FloatParam,

    #[id = "cutoff"]
    pub cutoff: FloatParam,

    #[id = "resonance"]
    pub resonance: FloatParam,

    #[id = "shape"]
    pub shape: FloatParam,

    #[id = "morph"]
    pub morph: FloatParam,

    #[id = "oscillator_type"]
    pub oscillator_type: EnumParam<OscillatorType>,
}

impl Default for Toby {
    fn default() -> Self {
        Self {
            params: Arc::new(TobyParams::default()),
            sample_rate: 1.0,

            phase: 0.0,

            midi_note_id: 0,
            midi_note_freq: 1.0,

            oscillator: OscillatorEngine::default(),
            filter: filter::Svf::default(),
            envelope: envelope::ADSR::default(),
        }
    }
}

impl Default for TobyParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new(
                "Gain",
                -10.0,
                FloatRange::Linear {
                    min: -30.0,
                    max: 0.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(3.0))
            .with_step_size(0.01)
            .with_unit(" dB"),

            cutoff: FloatParam::new(
                "Filter Cutoff",
                20_000.0,
                FloatRange::Skewed {
                    min: 1.0,
                    max: 20_000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            // We purposely don't specify a step size here, but the parameter should still be
            // displayed as if it were rounded. This formatter also includes the unit.
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0))
            .with_string_to_value(formatters::s2v_f32_hz_then_khz()),
            resonance: FloatParam::new(
                "Filter Resonance",
                0.5,
                FloatRange::Linear {
                    min: 0.01,
                    max: 100.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(3.0)),
            shape: FloatParam::new("Shape", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_smoother(SmoothingStyle::Linear(3.0))
                .with_step_size(0.01),
            morph: FloatParam::new("Morph", 0.2, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_smoother(SmoothingStyle::Linear(3.0))
                .with_step_size(0.01),

            oscillator_type: EnumParam::new("Oscillator Type", OscillatorType::SuperSquare),
        }
    }
}

impl Plugin for Toby {
    const NAME: &'static str = "Toby";
    const VENDOR: &'static str = "abstract audio";
    const URL: &'static str = "https://youtu.be/dQw4w9WgXcQ";
    const EMAIL: &'static str = "info@example.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            // This is also the default and can be omitted here
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;

        true
    }

    fn reset(&mut self) {
        self.phase = 0.0;
        self.midi_note_id = 0;
        self.midi_note_freq = 1.0;
        self.envelope.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let samples = buffer.samples();
        let mut next_event = context.next_event();
        for (sample_id, channel_samples) in buffer.iter_samples().enumerate() {
            // Smoothing is optionally built into the parameters themselves

            // This plugin can be either triggered by MIDI or controleld by a parameter
            while let Some(event) = next_event {
                // If the event occured after the sample_time, stop
                if event.timing() > sample_id as u32 {
                    break;
                }

                match event {
                    NoteEvent::NoteOn { note, velocity, .. } => {
                        self.midi_note_id = note;
                        self.midi_note_freq = util::midi_note_to_freq(note);

                        match self.envelope.stage {
                            EnvelopeStage::Attack | EnvelopeStage::Release => {
                                self.envelope.trigger(envelope::EnvelopeEvent::Attack);
                            }
                            EnvelopeStage::Decay | EnvelopeStage::Sustain => {
                                self.envelope.timer = 0.0;
                            }
                        }
                    }
                    NoteEvent::NoteOff { note, .. } if note == self.midi_note_id => {
                        self.envelope.trigger(envelope::EnvelopeEvent::Release)
                    }
                    _ => (),
                }

                next_event = context.next_event();
            }

            let shape = self.params.shape.smoothed.next_step(samples as u32);
            let morph = self.params.morph.smoothed.next_step(samples as u32);
            let osc_params = OscillatorParams { shape, morph };

            let oscillator_type = self.params.oscillator_type.value();
            self.oscillator.selected = oscillator_type;

            self.oscillator
                .prepare_block(osc_params, self.midi_note_freq, self.sample_rate);

            for sample in channel_samples {
                let gain = self.params.gain.smoothed.next();
                let cutoff = self.params.cutoff.smoothed.next();
                let resonance = self.params.resonance.smoothed.next();

                self.filter.set_f_q(cutoff / self.sample_rate, resonance);

                let v = self
                    .oscillator
                    .process(self.midi_note_freq, self.sample_rate);

                let v = v * self.envelope.next(self.sample_rate);

                let v = self.filter.process(v);

                *sample = v * util::db_to_gain_fast(gain);
            }
        }

        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for Toby {
    const CLAP_ID: &'static str = "com.abstractaudio.toby";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("An optionally MIDI controlled sine test tone");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
        ClapFeature::Mono,
    ];
}

impl Vst3Plugin for Toby {
    const VST3_CLASS_ID: [u8; 16] = *b"AbstractTobyToby";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nih_export_clap!(Toby);
nih_export_vst3!(Toby);

#[macro_export]
macro_rules! log {
    ($($args:tt)*) => (
        let mut f = unsafe { std::fs::File::from_raw_fd(2) };
        _ = write!(f,$($args)*)
    );
}
