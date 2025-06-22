mod envelope;
mod filter;
mod oscillator;
pub mod voice;

use envelope::EnvelopeStage;
use nih_plug::prelude::*;
use std::sync::Arc;

use crate::{
    oscillator::engine::{OscillatorEngine, OscillatorParams, OscillatorType},
    voice::{Voice, VoiceParams},
};

pub struct Toby {
    params: Arc<TobyParams>,
    sample_rate: f32,

    voices: [Voice; 6],
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

            voices: [
                Voice::new(),
                Voice::new(),
                Voice::new(),
                Voice::new(),
                Voice::new(),
                Voice::new(),
            ],
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
        for voice in self.voices.iter_mut() {
            voice.envelope.reset();
        }
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let samples = buffer.samples();
        let mut next_event = context.next_event();

        let shape = self.params.shape.smoothed.next_step(samples as u32);
        let morph = self.params.morph.smoothed.next_step(samples as u32);
        let gain = self.params.gain.smoothed.next_step(samples as u32);
        let cutoff = self.params.cutoff.smoothed.next_step(samples as u32);
        let oscillator_type = self.params.oscillator_type.value();
        let resonance = self.params.resonance.smoothed.next_step(samples as u32);

        let voice_params = VoiceParams {
            oscillator_type,
            shape,
            morph,
            gain,
            cutoff,
            resonance,
        };

        for voice in self.voices.iter_mut() {
            voice.prepare_block(voice_params, self.sample_rate);
        }

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
                        // multi-pass voice allocation
                        let mut found_voice = false;

                        // 1. find a voice with the same note id
                        for voice in self.voices.iter_mut() {
                            if voice.midi_note_id == note {
                                voice.trigger(note, velocity);
                                found_voice = true;
                                break;
                            }
                        }

                        // 2. find an inactive voice
                        if !found_voice {
                            for voice in self.voices.iter_mut() {
                                if !voice.is_active() {
                                    voice.trigger(note, velocity);
                                    found_voice = true;
                                    break;
                                }
                            }
                        }

                        // 3. voice stealing: prefer an releasing voice
                        if !found_voice {
                            for voice in self.voices.iter_mut() {
                                if voice.envelope.stage == EnvelopeStage::Release {
                                    voice.trigger(note, velocity);
                                    found_voice = true;
                                    break;
                                }
                            }
                        }

                        // 4. voice stealing: prefer the oldest voice
                        if !found_voice {
                            let victim = self
                                .voices
                                .iter_mut()
                                .max_by(|a, b| {
                                    a.envelope.timer.partial_cmp(&b.envelope.timer).unwrap()
                                })
                                .unwrap();
                            victim.trigger(note, velocity);
                        }
                    }
                    NoteEvent::NoteOff { note, .. } => {
                        for voice in self.voices.iter_mut() {
                            if voice.midi_note_id == note {
                                voice.release();
                                break;
                            }
                        }
                    }
                    _ => (),
                }

                next_event = context.next_event();
            }

            for sample in channel_samples {
                let mut v = 0.0;
                for voice in self.voices.iter_mut() {
                    v += voice.process(voice_params, self.sample_rate);
                }

                *sample = v;
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
