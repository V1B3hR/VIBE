use super::graph::{flush_denormal_f64, AudioBuffer, AudioProcessor, Parameter, ProcessingContext};
use uuid::Uuid;

pub struct Delay {
    #[allow(dead_code)]
    id: Uuid,
    buffer_l: Vec<f64>,
    buffer_r: Vec<f64>,
    write_pos: usize,
    pub time: f32, // 0.0 to 1.0
    pub feedback: f32,
    pub mix: f32,
}

impl Delay {
    pub fn new(time: f32, feedback: f32, mix: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            buffer_l: vec![0.0; 44100 * 2], // 2 seconds at 44.1kHz
            buffer_r: vec![0.0; 44100 * 2],
            write_pos: 0,
            time,
            feedback,
            mix,
        }
    }
}

impl AudioProcessor for Delay {
    fn process(&mut self, buffer: &mut AudioBuffer, context: &ProcessingContext) {
        let sample_rate = context.sample_rate;
        let delay_samples = (self.time as f64 * sample_rate) as usize;
        let frames = buffer.frames;

        let feedback = self.feedback as f64;
        let mix = self.mix as f64;

        for i in 0..frames {
            let read_pos = if self.write_pos >= delay_samples {
                self.write_pos - delay_samples
            } else {
                self.buffer_l.len() + self.write_pos - delay_samples
            };

            let read_pos = read_pos % self.buffer_l.len();

            // Left
            let dry_l = buffer.channels_data[0][i];
            let wet_l = self.buffer_l[read_pos];
            // Denormal protection in feedback path
            self.buffer_l[self.write_pos] = flush_denormal_f64(dry_l + wet_l * feedback);
            buffer.channels_data[0][i] = dry_l * (1.0 - mix) + wet_l * mix;

            // Right
            let dry_r = buffer.channels_data[1][i];
            let wet_r = self.buffer_r[read_pos];
            // Denormal protection in feedback path
            self.buffer_r[self.write_pos] = flush_denormal_f64(dry_r + wet_r * feedback);
            buffer.channels_data[1][i] = dry_r * (1.0 - mix) + wet_r * mix;

            self.write_pos = (self.write_pos + 1) % self.buffer_l.len();
        }
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        Box::new(super::graph::DummyProcessor {
            id: self.id,
            name: self.name(),
            parameters: Vec::new(),
        })
    }
}

/// High-pass filter (RBJ biquad implementation)
/// Currently unused in main processing chain – kept as ready building block for:
/// - Prisma EQ high-cut bands
/// - Multiband crossover
/// - Sidechain high-pass on kick/ducker
#[allow(dead_code)]
pub struct HighPassFilter {
    id: Uuid,
    pub cutoff: f32,
    prev_input: [f64; 2],
    prev_output: [f64; 2],
}

impl HighPassFilter {
    #[allow(dead_code)]
    pub fn new(cutoff: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            cutoff,
            prev_input: [0.0; 2],
            prev_output: [0.0; 2],
        }
    }
}

impl AudioProcessor for HighPassFilter {
    fn process(&mut self, buffer: &mut AudioBuffer, _context: &ProcessingContext) {
        let alpha = self.cutoff as f64;
        for c in 0..2 {
            for i in 0..buffer.frames {
                let sample = buffer.channels_data[c][i];
                let out = alpha * (self.prev_output[c] + sample - self.prev_input[c]);
                self.prev_input[c] = sample;
                // Denormal protection
                self.prev_output[c] = flush_denormal_f64(out);
                buffer.channels_data[c][i] = out;
            }
        }
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        Box::new(super::graph::DummyProcessor {
            id: self.id,
            name: self.name(),
            parameters: Vec::new(),
        })
    }
}

pub struct Saturation {
    #[allow(dead_code)]
    id: Uuid,
    pub drive: f32, // 1.0 to 10.0
}

impl Saturation {
    pub fn new(drive: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            drive,
        }
    }
}

impl AudioProcessor for Saturation {
    fn process(&mut self, buffer: &mut AudioBuffer, _context: &ProcessingContext) {
        let drive = self.drive as f64;
        for c in 0..buffer.num_channels {
            super::simd_optimized::apply_saturation_optimized(
                &mut buffer.channels_data[c][..buffer.frames],
                drive - 1.0, // apply_saturation_optimized expects 'warmth' where 1.0 is full drive
            );
        }
    }
    fn id(&self) -> Uuid {
        self.id
    }

    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        Box::new(super::graph::DummyProcessor {
            id: self.id,
            name: self.name(),
            parameters: Vec::new(),
        })
    }
}

// Re-export full Reverb implementation
pub use super::processors::Reverb;

pub struct MasterSafetyLimiter {
    id: Uuid,
    // POINT 3: TPDF Dithering for bit-depth conversion
    pub dither: Parameter,
    rng_state: u64, // Simple LCG for TPDF noise generation
}

impl MasterSafetyLimiter {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            dither: Parameter::new("Master Dither", 1.0, 0.0, 1.0),
            rng_state: 0x123456789ABCDEF0,
        }
    }

    /// Generate TPDF (Triangular Probability Density Function) noise
    /// Range: -1.0 to +1.0 LSB for 16-bit output
    fn generate_tpdf(&mut self) -> f64 {
        // Linear Congruential Generator (fast, deterministic)
        self.rng_state = self
            .rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        let r1 = (self.rng_state >> 32) as f64 / 4294967296.0;

        self.rng_state = self
            .rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        let r2 = (self.rng_state >> 32) as f64 / 4294967296.0;

        // TPDF = sum of two uniform random variables
        // Scaled to ±0.5 LSB for 16-bit (1/32768)
        (r1 + r2 - 1.0) * (1.0 / 65536.0)
    }
}

impl AudioProcessor for MasterSafetyLimiter {
    fn process(&mut self, buffer: &mut AudioBuffer, _context: &ProcessingContext) {
        for c in 0..buffer.num_channels {
            super::simd_optimized::apply_limiter_optimized(
                &mut buffer.channels_data[c][..buffer.frames],
            );

            // 3. TPDF Dithering (Point 3: Audiophile bit-depth conversion)
            if self.dither.get_current_value() > 0.5 {
                for i in 0..buffer.frames {
                    buffer.channels_data[c][i] += self.generate_tpdf();
                }
            }
        }
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> String {
        "Master Safety Limiter".to_string()
    }

    fn get_parameters(&mut self) -> Vec<&mut Parameter> {
        vec![&mut self.dither]
    }

    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        Box::new(Self {
            id: self.id,
            dither: self.dither.clone(),
            rng_state: self.rng_state,
        })
    }
}

#[allow(dead_code)]
pub struct PrismaEQ {
    id: Uuid,
    pub low_cut_freq: Parameter,
    pub bell_freq: Parameter,
    pub bell_gain: Parameter,
    pub bell_q: Parameter,
    pub high_shelf_freq: Parameter,
    pub high_shelf_gain: Parameter,

    // Filter states (Direct Form I or II)
    // [channel][band][state]
    filter_state: [[[f64; 2]; 4]; 2],
}

impl PrismaEQ {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            low_cut_freq: Parameter::new("Low Cut", 30.0, 20.0, 500.0),
            bell_freq: Parameter::new("Bell Freq", 1000.0, 100.0, 10000.0),
            bell_gain: Parameter::new("Bell Gain", 0.0, -12.0, 12.0),
            bell_q: Parameter::new("Bell Q", 1.0, 0.1, 10.0),
            high_shelf_freq: Parameter::new("High Shelf", 5000.0, 1000.0, 20000.0),
            high_shelf_gain: Parameter::new("High Shelf Gain", 0.0, -12.0, 12.0),
            filter_state: [[[0.0; 2]; 4]; 2],
        }
    }
}

impl AudioProcessor for PrismaEQ {
    fn process(&mut self, _buffer: &mut AudioBuffer, _context: &ProcessingContext) {
        // TODO: Implement RBJ Biquad filters calculation and processing
        // For now, passthrough
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        Box::new(super::graph::DummyProcessor {
            id: self.id,
            name: self.name(),
            parameters: vec![
                self.low_cut_freq.clone(),
                self.bell_freq.clone(),
                self.bell_gain.clone(),
                self.bell_q.clone(),
                self.high_shelf_freq.clone(),
                self.high_shelf_gain.clone(),
            ],
        })
    }

    fn name(&self) -> String {
        "Prisma EQ".to_string()
    }

    fn get_parameters(&mut self) -> Vec<&mut Parameter> {
        vec![
            &mut self.low_cut_freq,
            &mut self.bell_freq,
            &mut self.bell_gain,
            &mut self.bell_q,
            &mut self.high_shelf_freq,
            &mut self.high_shelf_gain,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::graph::AudioBuffer;

    #[test]
    fn highpass_basic() {
        let mut hp = HighPassFilter::new(0.5);
        let mut buffer = AudioBuffer::new();
        buffer.frames = 100;
        buffer.num_channels = 2;
        for c in 0..2 {
            for i in 0..100 {
                buffer.channels_data[c][i] = 1.0;
            }
        }
        let context = ProcessingContext {
            sample_rate: 44100.0,
            playhead: 0,
            sidechain: None,
        };
        hp.process(&mut buffer, &context);
        // High pass of DC step should decay
        assert!(buffer.channels_data[0][99].abs() < 1.0);
    }
}
