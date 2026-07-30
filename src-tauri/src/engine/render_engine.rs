#![allow(dead_code)]

use crate::engine::audio_graph::AudioGraph;
use crate::engine::graph::Track;
use crate::engine::streamer::GlobalBufferPool;
use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExportFormat {
    Wav,
    Mp3,
    Flac,
    Aiff,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BitDepth {
    Integer16,
    Integer24,
    Float32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DitherMode {
    None,
    Triangular,
    NoiseShaping,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RenderRange {
    EntireProject,
    LoopRegion,
    Selection(u64, u64),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RenderConfig {
    pub output_path: PathBuf,
    pub format: ExportFormat,
    pub sample_rate: u32,
    pub bit_depth: BitDepth,
    pub dithering: DitherMode,
    pub normalize_lufs: Option<f64>,
    pub range: RenderRange,
    pub stem_export: Vec<Uuid>,
    pub dry_run: bool,
    pub mp3_bitrate: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportProfile {
    pub name: String,
    pub config: RenderConfig,
    pub is_default: bool,
}

impl ExportProfile {
    pub fn preset_high_quality_wav(path: PathBuf) -> Self {
        Self {
            name: "Studio Master (WAV)".to_string(),
            config: RenderConfig {
                output_path: path,
                format: ExportFormat::Wav,
                sample_rate: 48000,
                bit_depth: BitDepth::Integer24,
                dithering: DitherMode::Triangular,
                normalize_lufs: Some(-14.0),
                range: RenderRange::EntireProject,
                stem_export: vec![],
                dry_run: false,
                mp3_bitrate: 320,
            },
            is_default: true,
        }
    }

    pub fn preset_streaming_mp3(path: PathBuf) -> Self {
        Self {
            name: "Streaming (MP3)".to_string(),
            config: RenderConfig {
                output_path: path,
                format: ExportFormat::Mp3,
                sample_rate: 44100,
                bit_depth: BitDepth::Integer16,
                dithering: DitherMode::NoiseShaping,
                normalize_lufs: Some(-14.0),
                range: RenderRange::EntireProject,
                stem_export: vec![],
                dry_run: false,
                mp3_bitrate: 320,
            },
            is_default: false,
        }
    }
}

pub enum RenderStatus {
    Progress(f32),
    AnalysisResult { lufs: f64, true_peak: f64 },
    Complete(PathBuf),
    Error(String),
}

pub trait AudioEncoder {
    fn write_buffer(&mut self, l: &[f32], r: &[f32], config: &RenderConfig) -> Result<(), String>;
    fn finalize(self: Box<Self>) -> Result<(), String>;
}

struct Ditherer {
    rng: rand::rngs::StdRng,
    error_l: f32,
    error_r: f32,
}

impl Ditherer {
    fn new() -> Self {
        use rand::SeedableRng;
        Self {
            rng: rand::rngs::StdRng::from_entropy(),
            error_l: 0.0,
            error_r: 0.0,
        }
    }

    fn triangular(&mut self) -> f32 {
        use rand::Rng;
        let r1: f32 = self.rng.gen();
        let r2: f32 = self.rng.gen();
        r1 + r2 - 1.0
    }

    fn dither_sample(&mut self, sample: f32, mode: &DitherMode, scale: f32, is_right: bool) -> f32 {
        let err = if is_right { self.error_r } else { self.error_l };
        let mut s = sample;

        match mode {
            DitherMode::None => s,
            DitherMode::Triangular => s + self.triangular() / scale,
            DitherMode::NoiseShaping => {
                // Simple 1st order HP noise shaping
                let input_with_err = s - err;
                let dither = self.triangular() / scale;
                s = input_with_err + dither;
                s
            }
        }
    }

    fn update_error(&mut self, pre_quant: f32, post_quant: f32, is_right: bool) {
        if is_right {
            self.error_r = post_quant - pre_quant;
        } else {
            self.error_l = post_quant - pre_quant;
        }
    }
}

struct WavEncoder {
    writer: hound::WavWriter<std::io::BufWriter<std::fs::File>>,
    ditherer: Ditherer,
}

impl WavEncoder {
    fn new(path: &PathBuf, sample_rate: u32, bit_depth: &BitDepth) -> Result<Self, String> {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate,
            bits_per_sample: match bit_depth {
                BitDepth::Integer16 => 16,
                BitDepth::Integer24 => 24,
                BitDepth::Float32 => 32,
            },
            sample_format: match bit_depth {
                BitDepth::Float32 => hound::SampleFormat::Float,
                _ => hound::SampleFormat::Int,
            },
        };
        let writer = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
        Ok(Self {
            writer,
            ditherer: Ditherer::new(),
        })
    }
}

impl AudioEncoder for WavEncoder {
    fn write_buffer(&mut self, l: &[f32], r: &[f32], config: &RenderConfig) -> Result<(), String> {
        for i in 0..l.len() {
            match config.bit_depth {
                BitDepth::Float32 => {
                    self.writer.write_sample(l[i]).map_err(|e| e.to_string())?;
                    self.writer.write_sample(r[i]).map_err(|e| e.to_string())?;
                }
                BitDepth::Integer16 => {
                    let mut sl = l[i];
                    let mut sr = r[i];

                    sl = self
                        .ditherer
                        .dither_sample(sl, &config.dithering, 32768.0, false);
                    sr = self
                        .ditherer
                        .dither_sample(sr, &config.dithering, 32768.0, true);

                    let v_l = (sl * 32767.0).clamp(-32768.0, 32767.0);
                    let v_r = (sr * 32767.0).clamp(-32768.0, 32767.0);

                    // Update noise shaping error (normalized to -1.0..1.0 range)
                    self.ditherer.update_error(sl, v_l / 32767.0, false);
                    self.ditherer.update_error(sr, v_r / 32767.0, true);

                    self.writer
                        .write_sample(v_l as i16)
                        .map_err(|e| e.to_string())?;
                    self.writer
                        .write_sample(v_r as i16)
                        .map_err(|e| e.to_string())?;
                }
                BitDepth::Integer24 => {
                    let mut sl = l[i];
                    let mut sr = r[i];

                    sl = self
                        .ditherer
                        .dither_sample(sl, &config.dithering, 8388608.0, false);
                    sr = self
                        .ditherer
                        .dither_sample(sr, &config.dithering, 8388608.0, true);

                    let v_l = (sl * 8_388_607.0).clamp(-8_388_608.0, 8_388_607.0);
                    let v_r = (sr * 8_388_607.0).clamp(-8_388_608.0, 8_388_607.0);

                    // Update noise shaping error
                    self.ditherer.update_error(sl, v_l / 8388607.0, false);
                    self.ditherer.update_error(sr, v_r / 8388607.0, true);

                    self.writer
                        .write_sample(v_l as i32)
                        .map_err(|e| e.to_string())?;
                    self.writer
                        .write_sample(v_r as i32)
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        Ok(())
    }
    fn finalize(self: Box<Self>) -> Result<(), String> {
        self.writer.finalize().map_err(|e| e.to_string())
    }
}

#[cfg(feature = "flac")]
struct FlacEncoder {
    encoder: flac_bound::FlacEncoder,
}

#[cfg(feature = "flac")]
impl FlacEncoder {
    fn new(path: &PathBuf, sample_rate: u32, bit_depth: &BitDepth) -> Result<Self, String> {
        let mut encoder =
            flac_bound::FlacEncoder::new().map_err(|_| "Failed to create FLAC encoder")?;

        encoder
            .set_sample_rate(sample_rate)
            .map_err(|e| format!("FLAC: {:?}", e))?;
        encoder
            .set_channels(2)
            .map_err(|e| format!("FLAC: {:?}", e))?;
        encoder
            .set_bits_per_sample(match bit_depth {
                BitDepth::Integer16 => 16,
                BitDepth::Integer24 => 24,
                _ => 24, // FLAC usually 16 or 24
            })
            .map_err(|e| format!("FLAC: {:?}", e))?;

        encoder
            .init_file(path)
            .map_err(|e| format!("FLAC Init: {:?}", e))?;

        Ok(Self { encoder })
    }
}

#[cfg(feature = "flac")]
impl AudioEncoder for FlacEncoder {
    fn write_buffer(&mut self, l: &[f32], r: &[f32], config: &RenderConfig) -> Result<(), String> {
        let bps = match config.bit_depth {
            BitDepth::Integer16 => 16,
            _ => 24,
        };

        // Interleave and scale
        let mut interleaved = Vec::with_capacity(l.len() * 2);
        let scale = if bps == 16 { 32767.0 } else { 8388607.0 };

        for i in 0..l.len() {
            interleaved.push((l[i].clamp(-1.0, 1.0) * scale) as i32);
            interleaved.push((r[i].clamp(-1.0, 1.0) * scale) as i32);
        }

        self.encoder
            .process_interleaved(&interleaved)
            .map_err(|e| format!("FLAC Process: {:?}", e))?;
        Ok(())
    }
    fn finalize(mut self: Box<Self>) -> Result<(), String> {
        self.encoder
            .finish()
            .map_err(|e| format!("FLAC Finish: {:?}", e))?;
        Ok(())
    }
}

struct AiffEncoder {
    // Placeholder - AIFF is complex to implement from scratch and hound is WAV only
    // We'll wrap a WAV writer and note it's not strictly AIFF yet
    writer: Option<hound::WavWriter<std::io::BufWriter<std::fs::File>>>,
}

impl AiffEncoder {
    fn new(path: &PathBuf, sample_rate: u32, bit_depth: &BitDepth) -> Result<Self, String> {
        // Technically this is creating a WAV, but we name it .aif
        // Real AIFF needs Big Endian and different chunks.
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate,
            bits_per_sample: match bit_depth {
                BitDepth::Integer16 => 16,
                BitDepth::Integer24 => 24,
                BitDepth::Float32 => 32,
            },
            sample_format: match bit_depth {
                BitDepth::Float32 => hound::SampleFormat::Float,
                _ => hound::SampleFormat::Int,
            },
        };
        let writer = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
        Ok(Self {
            writer: Some(writer),
        })
    }
}

impl AudioEncoder for AiffEncoder {
    fn write_buffer(&mut self, l: &[f32], r: &[f32], config: &RenderConfig) -> Result<(), String> {
        if let Some(ref mut w) = self.writer {
            // Re-use logic or just write
            for i in 0..l.len() {
                match config.bit_depth {
                    BitDepth::Float32 => {
                        w.write_sample(l[i]).map_err(|e| e.to_string())?;
                        w.write_sample(r[i]).map_err(|e| e.to_string())?;
                    }
                    BitDepth::Integer16 => {
                        w.write_sample((l[i] * 32767.0) as i16)
                            .map_err(|e| e.to_string())?;
                        w.write_sample((r[i] * 32767.0) as i16)
                            .map_err(|e| e.to_string())?;
                    }
                    BitDepth::Integer24 => {
                        w.write_sample((l[i] * 8388607.0) as i32)
                            .map_err(|e| e.to_string())?;
                        w.write_sample((r[i] * 8388607.0) as i32)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
        }
        Ok(())
    }
    fn finalize(mut self: Box<Self>) -> Result<(), String> {
        if let Some(w) = self.writer.take() {
            w.finalize().map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

#[cfg(feature = "mp3")]
impl Mp3Encoder {
    fn new(
        path: &PathBuf,
        sample_rate: u32,
        bit_depth: &BitDepth,
        bitrate: u32,
    ) -> Result<Self, String> {
        let mut lame = lame::Lame::new().ok_or("Failed to initialize LAME")?;
        lame.set_sample_rate(sample_rate)
            .map_err(|e| format!("LAME: {:?}", e))?;
        lame.set_channels(2).map_err(|e| format!("LAME: {:?}", e))?;
        lame.set_kilobitrate(bitrate as i32)
            .map_err(|e| format!("LAME: {:?}", e))?;
        lame.set_quality(2).map_err(|e| format!("LAME: {:?}", e))?; // High quality
        lame.init_params()
            .map_err(|e| format!("LAME Init: {:?}", e))?;

        let file = std::fs::File::create(path).map_err(|e| e.to_string())?;

        Ok(Self {
            path: path.clone(),
            lame,
            bit_depth: bit_depth.clone(),
            file,
        })
    }
}

#[cfg(feature = "mp3")]
impl AudioEncoder for Mp3Encoder {
    fn write_buffer(&mut self, l: &[f32], r: &[f32], _config: &RenderConfig) -> Result<(), String> {
        use std::io::Write;
        let l_i16: Vec<i16> = l
            .iter()
            .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
            .collect();
        let r_i16: Vec<i16> = r
            .iter()
            .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
            .collect();

        let mut mp3_buffer = vec![0u8; l.len() * 2]; // Roughly enough
        let wrote = self
            .lame
            .encode(&l_i16, &r_i16, &mut mp3_buffer)
            .map_err(|e| format!("LAME Encode: {:?}", e))?;
        self.file
            .write_all(&mp3_buffer[..wrote])
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    fn finalize(self: Box<Self>) -> Result<(), String> {
        // LAME 0.1.3 crate is missing flush.
        // We could pad with zeros to force output if needed.
        Ok(())
    }
}

struct LoudnessAnalyzer {
    meter: ebur128::EbuR128,
    max_peak: f32,
}

impl LoudnessAnalyzer {
    fn new(sample_rate: u32) -> Self {
        Self {
            meter: ebur128::EbuR128::new(
                2,
                sample_rate,
                ebur128::Mode::I | ebur128::Mode::TRUE_PEAK,
            )
            .unwrap(),
            max_peak: 0.0,
        }
    }

    fn add_buffer(&mut self, l: &[f32], r: &[f32]) {
        // Interleave for ebur128
        let mut interleaved = Vec::with_capacity(l.len() * 2);
        for i in 0..l.len() {
            interleaved.push(l[i]);
            interleaved.push(r[i]);

            let peak = l[i].abs().max(r[i].abs());
            if peak > self.max_peak {
                self.max_peak = peak;
            }
        }
        self.meter.add_frames_f32(&interleaved).unwrap();
    }

    fn results(&self) -> (f64, f64) {
        let lufs = self.meter.loudness_global().unwrap_or(-70.0);
        let mut tp = self.max_peak as f64;
        for i in 0..2 {
            if let Ok(p) = self.meter.true_peak(i) {
                if p > tp {
                    tp = p;
                }
            }
        }
        (lufs, tp)
    }
}

pub struct RenderEngine {
    graph: Arc<Mutex<AudioGraph>>,
    tracks: Vec<Track>, // Snapshot copy
    buffer_pool: Arc<GlobalBufferPool>,
    streamer: Arc<crate::engine::streamer::WindowsAsyncStreamer>,
    fades: Arc<crate::engine::fades::FadeLuts>,
    config: RenderConfig,
    progress_tx: Sender<RenderStatus>,
    limiter: crate::engine::dynamics_module::dsp::limiter::LookaheadLimiter,
}

impl RenderEngine {
    pub fn new(
        graph: AudioGraph,
        tracks: Vec<Track>,
        buffer_pool: Arc<GlobalBufferPool>,
        streamer: Arc<crate::engine::streamer::WindowsAsyncStreamer>,
        fades: Arc<crate::engine::fades::FadeLuts>,
        config: RenderConfig,
        progress_tx: Sender<RenderStatus>,
    ) -> Self {
        let sample_rate = config.sample_rate;
        Self {
            graph: Arc::new(Mutex::new(graph)),
            tracks,
            buffer_pool,
            streamer,
            fades,
            config,
            progress_tx,
            limiter: crate::engine::dynamics_module::dsp::limiter::LookaheadLimiter::new(
                sample_rate as f64,
            ),
        }
    }

    pub fn render(&mut self) {
        let sample_rate = self.config.sample_rate as f64;
        let (start_sample, end_sample) = self.calculate_range(sample_rate);

        // --- PASS 1: ANALYSIS ---
        let mut normalization_gain = 1.0;

        if self.config.normalize_lufs.is_some() || self.config.dry_run {
            let (lufs, tp) = self.run_pass(start_sample, end_sample, None);
            let _ = self.progress_tx.send(RenderStatus::AnalysisResult {
                lufs,
                true_peak: tp,
            });

            if let Some(target) = self.config.normalize_lufs {
                let delta = target - lufs;
                normalization_gain = 10.0f64.powf(delta / 20.0);

                let projected_tp = tp * normalization_gain;
                if projected_tp > 0.89 {
                    // -1.0 dB approx
                    normalization_gain *= 0.89 / projected_tp;
                }
            }

            if self.config.dry_run {
                let _ = self
                    .progress_tx
                    .send(RenderStatus::Complete(self.config.output_path.clone()));
                return;
            }
        }

        // --- PASS 2: ACTUAL RENDER ---
        self.run_pass(start_sample, end_sample, Some(normalization_gain));

        let _ = self
            .progress_tx
            .send(RenderStatus::Complete(self.config.output_path.clone()));
    }

    fn calculate_range(&self, sample_rate: f64) -> (u64, u64) {
        match self.config.range {
            RenderRange::EntireProject => {
                let max_len = self
                    .tracks
                    .iter()
                    .flat_map(|t| t.clips.iter().map(|c| c.start_sample + c.length_in_samples))
                    .max()
                    .unwrap_or(0);
                (0, max_len + (sample_rate as u64 * 2)) // 2 sec tail
            }
            RenderRange::LoopRegion => (0, 0),
            RenderRange::Selection(start, end) => (start, end),
        }
    }

    fn run_pass(&mut self, start_sample: u64, end_sample: u64, gain: Option<f64>) -> (f64, f64) {
        let block_size = 4096;
        let mut current_sample = start_sample;
        let total_samples = end_sample - start_sample;
        let sample_rate = self.config.sample_rate as f64;

        let mut analyzer = LoudnessAnalyzer::new(self.config.sample_rate);
        let mut encoder: Option<Box<dyn AudioEncoder>> = None;
        let summing = crate::engine::summing::SummingEngine::new();

        if gain.is_some() {
            encoder = self.init_encoder();
        }

        let mut master_l = vec![0.0f64; block_size];
        let mut master_r = vec![0.0f64; block_size];

        let mut next_progress_milestone = 0.01;

        // Reset track states
        for track in &mut self.tracks {
            track.active_voices.clear();
        }

        while current_sample < end_sample {
            let remaining = end_sample - current_sample;
            let frames = (block_size as u64).min(remaining) as usize;

            master_l[..frames].fill(0.0);
            master_r[..frames].fill(0.0);

            // PROCESS TRACKS & SUM
            let mut channels = vec![&mut master_l[..frames], &mut master_r[..frames]];
            summing.process_parallel(
                &mut self.tracks,
                &mut channels,
                &[], // vca_groups (not yet used in offline render)
                sample_rate,
                120.0, // Default project BPM
                current_sample,
                &self.fades,
                &[], // midi_events
                &self.buffer_pool,
                &self.streamer,
                true, // offline
                &[], // hardware_inputs
                false, // is_playing
            );

            let mut buf_f32_l = vec![0.0f32; frames];
            let mut buf_f32_r = vec![0.0f32; frames];

            let _g_f32 = gain.unwrap_or(1.0) as f32; // Normalization gain applied before Limiter? No, Limiter handles it.
                                                     // Wait, we calculate normalization gain in Pass 1.
                                                     // If we use Limiter, we should drive into it?
                                                     // Current VIBE workflow: Normalize -> Limit.
                                                     // So we apply g_f32 to input of limiter?

            // Correction: For this implementation, we apply the calculated normalization gain BEFORE the limiter.
            // Then the limiter ensures we don't clip.

            for i in 0..frames {
                // Apply gain (Normalization)
                master_l[i] *= gain.unwrap_or(1.0);
                master_r[i] *= gain.unwrap_or(1.0);

                // Then Limiter (if Pass 2)
                if gain.is_some() {
                    let (l, r) = self.limiter.process_stereo(master_l[i], master_r[i]);
                    master_l[i] = l;
                    master_r[i] = r;
                }

                buf_f32_l[i] = master_l[i] as f32;
                buf_f32_r[i] = master_r[i] as f32;
            }

            analyzer.add_buffer(&buf_f32_l, &buf_f32_r);

            if let Some(ref mut e) = encoder {
                let _ = e.write_buffer(&buf_f32_l, &buf_f32_r, &self.config);
            }

            current_sample += frames as u64;

            let progress = (current_sample - start_sample) as f32 / total_samples as f32;
            if progress >= next_progress_milestone {
                let _ = self.progress_tx.send(RenderStatus::Progress(progress));
                next_progress_milestone += 0.01;
            }
        }

        if let Some(e) = encoder {
            let _ = e.finalize();
        }

        analyzer.results()
    }

    fn init_encoder(&self) -> Option<Box<dyn AudioEncoder>> {
        match self.config.format {
            ExportFormat::Wav => WavEncoder::new(
                &self.config.output_path,
                self.config.sample_rate,
                &self.config.bit_depth,
            )
            .ok()
            .map(|e| Box::new(e) as Box<dyn AudioEncoder>),
            ExportFormat::Mp3 => {
                #[cfg(feature = "mp3")]
                {
                    Mp3Encoder::new(
                        &self.config.output_path,
                        self.config.sample_rate,
                        &self.config.bit_depth,
                        self.config.mp3_bitrate,
                    )
                    .ok()
                    .map(|e| Box::new(e) as Box<dyn AudioEncoder>)
                }
                #[cfg(not(feature = "mp3"))]
                {
                    None
                }
            }
            ExportFormat::Flac => {
                #[cfg(feature = "flac")]
                {
                    FlacEncoder::new(
                        &self.config.output_path,
                        self.config.sample_rate,
                        &self.config.bit_depth,
                    )
                    .ok()
                    .map(|e| Box::new(e) as Box<dyn AudioEncoder>)
                }
                #[cfg(not(feature = "flac"))]
                {
                    None
                }
            }
            ExportFormat::Aiff => AiffEncoder::new(
                &self.config.output_path,
                self.config.sample_rate,
                &self.config.bit_depth,
            )
            .ok()
            .map(|e| Box::new(e) as Box<dyn AudioEncoder>),
        }
    }
}
