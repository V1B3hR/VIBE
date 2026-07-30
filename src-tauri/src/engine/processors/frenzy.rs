#![allow(dead_code)]
use super::super::graph::{
    flush_denormal_f64, AudioBuffer, AudioProcessor, Parameter, ProcessingContext,
};
use uuid::Uuid;

/// Frenzy Multiplier - A "Gemini Reaper" inspired multi-tap effect.
/// Multiplies the input signal into 1-8 "Frenzy" voices, each with its own
/// pitch, warmth, ice, and space processing.
pub struct FrenzyMultiplier {
    id: Uuid,

    // Global Parameters
    pub multiplier: Parameter,  // 1 to 8 voices
    pub scatter: Parameter,     // Timing dispersion (0-500ms)
    pub pitch_chaos: Parameter, // Random pitch deviation
    pub warmth: Parameter,      // Global saturation drive
    pub ice: Parameter,         // Global bitcrush/SRR
    pub space: Parameter,       // Global reverb mix

    // Internal State
    buffer: Vec<f64>, // Circular delay buffer for scatter
    write_pos: usize,
    sample_rate: f64,

    // Voice-specific state (8 voices)
    voices: Vec<FrenzyVoice>,
    rng_state: u64,
}

struct FrenzyVoice {
    read_pos_offset: f64, // Current read position relative to write_pos
    pitch_ratio: f64,

    // Internal filter/process states
    prev_sample: f64, // For SRR (Ice)
    srr_acc: f64,     // Sample rate reduction accumulator

    // Simple internal reverb state (one-pole + delay)
    reverb_buffer: Vec<f64>,
    reverb_pos: usize,
}

impl FrenzyMultiplier {
    pub fn new() -> Self {
        let mut voices = Vec::with_capacity(8);
        for i in 0..8 {
            voices.push(FrenzyVoice {
                read_pos_offset: 0.0,
                pitch_ratio: 1.0,
                prev_sample: 0.0,
                srr_acc: 0.0,
                reverb_buffer: vec![0.0; 4410 + i * 123], // Small unique delay lines
                reverb_pos: 0,
            });
        }

        Self {
            id: Uuid::new_v4(),
            multiplier: Parameter::new("Frenzy Count", 4.0, 1.0, 8.0),
            scatter: Parameter::new("Scatter", 20.0, 0.0, 500.0), // ms
            pitch_chaos: Parameter::new("Pitch Chaos", 0.1, 0.0, 2.0),
            warmth: Parameter::new("Warmth", 0.2, 0.0, 1.0),
            ice: Parameter::new("Ice", 0.0, 0.0, 1.0),
            space: Parameter::new("Space", 0.2, 0.0, 1.0),

            buffer: vec![0.0; 48000 * 2], // 2 seconds buffer
            write_pos: 0,
            sample_rate: 44100.0,
            voices,
            rng_state: 0xDEADBEEF,
        }
    }

    fn next_random(&mut self) -> f64 {
        self.rng_state = self
            .rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        (self.rng_state >> 32) as f64 / 4294967296.0
    }
}

impl AudioProcessor for FrenzyMultiplier {
    fn process(&mut self, buffer: &mut AudioBuffer, context: &ProcessingContext) {
        let sr = context.sample_rate;
        self.sample_rate = sr;
        let frames = buffer.frames;

        let count = self.multiplier.get_current_value() as usize;
        let scatter_ms = self.scatter.get_current_value();
        let pitch_chaos = self.pitch_chaos.get_current_value();
        let warmth_drive = 1.0 + self.warmth.get_current_value() * 10.0;
        let ice_factor = self.ice.get_current_value();
        let space_mix = self.space.get_current_value();

        let scatter_samples = scatter_ms * 0.001 * sr;

        for i in 0..frames {
            // Mono sum for input to multiplier
            let input_l = buffer.channels_data[0][i];
            let input_r = buffer.channels_data[1][i];
            let mono_in = (input_l + input_r) * 0.5;

            // Write to circular buffer
            self.buffer[self.write_pos] = mono_in;

            let mut out_l = 0.0;
            let mut out_r = 0.0;

            // Process active voices
            for v_idx in 0..count {
                // Call next_random before borrowing the voice mutably
                let rand_val = if i == 0 {
                    Some(self.next_random())
                } else {
                    None
                };

                let voice = &mut self.voices[v_idx];

                // 1. Calculate Read Position with Pitch Shift & Scatter
                // We use a simple linear interpolation for pitch shifting
                // Initial offset is based on scatter
                if let Some(r) = rand_val {
                    // Update chaos once per block
                    let rand_scatter = (v_idx as f64 / 8.0) * scatter_samples;
                    voice.pitch_ratio = 1.0 + (r - 0.5) * pitch_chaos * 0.2;
                    voice.read_pos_offset = rand_scatter;
                }

                // Advance read pointer based on pitch
                voice.read_pos_offset += voice.pitch_ratio - 1.0;

                let read_pos = (self.write_pos as f64 - voice.read_pos_offset
                    + self.buffer.len() as f64)
                    % self.buffer.len() as f64;
                let idx_a = read_pos as usize % self.buffer.len();
                let idx_b = (idx_a + 1) % self.buffer.len();
                let frac = read_pos.fract();

                let mut sample = self.buffer[idx_a] * (1.0 - frac) + self.buffer[idx_b] * frac;

                // 2. ICE (Sample Rate Reduction / Bitcrush)
                if ice_factor > 0.0 {
                    voice.srr_acc += ice_factor;
                    if voice.srr_acc >= 1.0 {
                        voice.srr_acc -= 1.0;
                        voice.prev_sample = sample;
                    }
                    sample = voice.prev_sample;

                    // Bitcrush (quantization)
                    let bits = 16.0 - (ice_factor * 12.0);
                    let levels = 2.0f64.powf(bits);
                    sample = (sample * levels).round() / levels;
                }

                // 3. WARMTH (Tanh Saturation)
                sample = (sample * warmth_drive).tanh();

                // 4. SPACE (Simple individual reverb/diffuser)
                if space_mix > 0.0 {
                    let rev_out = voice.reverb_buffer[voice.reverb_pos];
                    voice.reverb_buffer[voice.reverb_pos] =
                        flush_denormal_f64(sample + rev_out * 0.6);
                    voice.reverb_pos = (voice.reverb_pos + 1) % voice.reverb_buffer.len();
                    sample = sample * (1.0 - space_mix) + rev_out * space_mix;
                }

                // Pan voices across stereo field
                let pan = (v_idx as f64 / (count.max(2) - 1) as f64) * 2.0 - 1.0;
                let gain_l = (1.0 - pan).sqrt();
                let gain_r = (1.0 + pan).sqrt();

                out_l += sample * gain_l;
                out_r += sample * gain_r;
            }

            // Normalize sum relative to voice count to prevent clipping
            let norm = 1.0 / (count as f64).sqrt().max(1.0);
            buffer.channels_data[0][i] = out_l * norm;
            buffer.channels_data[1][i] = out_r * norm;

            self.write_pos = (self.write_pos + 1) % self.buffer.len();
        }
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> String {
        "Frenzy Multiplier".to_string()
    }

    fn get_parameters(&mut self) -> Vec<&mut Parameter> {
        vec![
            &mut self.multiplier,
            &mut self.scatter,
            &mut self.pitch_chaos,
            &mut self.warmth,
            &mut self.ice,
            &mut self.space,
        ]
    }

    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        // Fallback or full clone depending on complexity
        // For VIBE engine, DummyProcessor is often used for UI-only clones
        Box::new(super::super::graph::DummyProcessor {
            id: self.id,
            name: self.name(),
            parameters: vec![
                self.multiplier.clone(),
                self.scatter.clone(),
                self.pitch_chaos.clone(),
                self.warmth.clone(),
                self.ice.clone(),
                self.space.clone(),
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::graph::AudioBuffer;

    #[test]
    fn test_frenzy_processing() {
        let mut frenzy = FrenzyMultiplier::new();
        let mut buffer = AudioBuffer::new();
        buffer.frames = 128;
        buffer.num_channels = 2;

        // Impulse test
        buffer.channels_data[0][0] = 1.0;
        buffer.channels_data[1][0] = 1.0;

        let context = ProcessingContext {
            sample_rate: 44100.0,
            playhead: 0,
            sidechain: None,
        };

        frenzy.process(&mut buffer, &context);

        // Should have non-zero output
        let mut has_output = false;
        for i in 0..128 {
            if buffer.channels_data[0][i].abs() > 0.0 {
                has_output = true;
                break;
            }
        }
        assert!(has_output);
    }

    #[test]
    fn test_abyss_stability() {
        let mut frenzy = FrenzyMultiplier::new();

        // Push everything to the absolute limit (Depth of the Abyss)
        frenzy.multiplier.set_value(8.0);
        frenzy.scatter.set_value(500.0);
        frenzy.pitch_chaos.set_value(2.0);
        frenzy.warmth.set_value(1.0);
        frenzy.ice.set_value(1.0);
        frenzy.space.set_value(1.0);

        let mut buffer = AudioBuffer::new();
        buffer.frames = 1024;
        buffer.num_channels = 2;

        // Feed a harsh 440Hz square wave to stress the filters/saturators
        for i in 0..1024 {
            let val = if (i / 50) % 2 == 0 { 1.0 } else { -1.0 };
            buffer.channels_data[0][i] = val;
            buffer.channels_data[1][i] = val;
        }

        let context = ProcessingContext {
            sample_rate: 44100.0,
            playhead: 0,
            sidechain: None,
        };

        // Process a few blocks to let reverb tails and delays build up
        for _ in 0..10 {
            frenzy.process(&mut buffer, &context);

            // Critical Safety Checks
            for c in 0..2 {
                for i in 0..1024 {
                    let sample = buffer.channels_data[c][i];

                    // 1. No NaNs or Infs allowed in the abyss
                    assert!(sample.is_finite(), "Abyss produced non-finite value!");

                    // 2. Normalization check: Should stay within reasonable bounds (+/- 12dB approx)
                    // (Even with 8 voices, normalization should keep it near 1.0)
                    assert!(sample.abs() < 4.0, "Abyss overflow! Sample: {}", sample);
                }
            }
        }
    }
}
