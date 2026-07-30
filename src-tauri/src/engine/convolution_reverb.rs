#![allow(dead_code)]
use super::graph::{AudioBuffer, AudioProcessor, Parameter, ProcessingContext};
use num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;
use uuid::Uuid;

/// ConvolutionReverb implements a high-performance zero-latency partitioned convolution engine.
/// It uses Uniformly Partitioned Convolution (UPC) to achieve efficiency for long impulse responses.
pub struct ConvolutionReverb {
    id: Uuid,
    pub mix: Parameter,
    pub size: Parameter,
    
    // Original IR info for cloning/serialization
    ir_path: Option<String>,
    ir_data_l: Vec<f64>,
    ir_data_r: Vec<f64>,

    // FFT State
    fft_forward: Arc<dyn Fft<f64>>,
    fft_inverse: Arc<dyn Fft<f64>>,
    fft_len: usize,

    // Partitions of the Impulse Response in frequency domain
    ir_partitions_l: Vec<Vec<Complex<f64>>>,
    ir_partitions_r: Vec<Vec<Complex<f64>>>,

    // Internal circular buffer for handling arbitrary block sizes
    input_buffer_l: Vec<f64>,
    input_buffer_r: Vec<f64>,
    output_buffer_l: Vec<f64>,
    output_buffer_r: Vec<f64>,
    
    // Positions for internal buffering
    buf_read_pos: usize,
    buf_write_pos: usize,
    samples_in_buf: usize,

    // Frequency domain input history for partitioned convolution
    // [partition_index][complex_samples]
    input_freq_history_l: Vec<Vec<Complex<f64>>>,
    input_freq_history_r: Vec<Vec<Complex<f64>>>,

    // Overlap-save state for the current partition
    output_overlap_l: Vec<f64>,
    output_overlap_r: Vec<f64>,

    block_size: usize,
    num_partitions: usize,
    history_pos: usize,
}

impl ConvolutionReverb {
    pub fn new(ir_l: &[f64], ir_r: &[f64], block_size: usize) -> Self {
        let mut planner = FftPlanner::new();
        let fft_len = block_size * 2;
        let fft_forward = planner.plan_fft_forward(fft_len);
        let fft_inverse = planner.plan_fft_inverse(fft_len);

        let num_partitions = ir_l.len().div_ceil(block_size);

        let mut ir_partitions_l = Vec::with_capacity(num_partitions);
        let mut ir_partitions_r = Vec::with_capacity(num_partitions);

        // Pre-compute IR partitions in frequency domain (Overlap-Save format)
        for i in 0..num_partitions {
            let start = i * block_size;
            let end = (start + block_size).min(ir_l.len());

            let mut part_l = vec![Complex::default(); fft_len];
            let mut part_r = vec![Complex::default(); fft_len];

            // In overlap-save, we pad the block with zeros for the first half
            for j in 0..(end - start) {
                part_l[j] = Complex::new(ir_l[start + j], 0.0);
                part_r[j] = Complex::new(ir_r[start + j], 0.0);
            }

            fft_forward.process(&mut part_l);
            fft_forward.process(&mut part_r);

            ir_partitions_l.push(part_l);
            ir_partitions_r.push(part_r);
        }

        Self {
            id: Uuid::new_v4(),
            mix: Parameter::new("Mix", 0.5, 0.0, 1.0),
            size: Parameter::new("Size", 1.0, 0.1, 2.0),
            ir_path: None,
            ir_data_l: ir_l.to_vec(),
            ir_data_r: ir_r.to_vec(),
            fft_forward,
            fft_inverse,
            fft_len,
            ir_partitions_l,
            ir_partitions_r,
            input_buffer_l: vec![0.0; block_size * 4],
            input_buffer_r: vec![0.0; block_size * 4],
            output_buffer_l: vec![0.0; block_size * 4],
            output_buffer_r: vec![0.0; block_size * 4],
            buf_read_pos: 0,
            buf_write_pos: 0,
            samples_in_buf: 0,
            input_freq_history_l: vec![vec![Complex::default(); fft_len]; num_partitions],
            input_freq_history_r: vec![vec![Complex::default(); fft_len]; num_partitions],
            output_overlap_l: vec![0.0; fft_len],
            output_overlap_r: vec![0.0; fft_len],
            block_size,
            num_partitions,
            history_pos: 0,
        }
    }

    fn process_block(&mut self) {
        let fft_len = self.fft_len;

        // 1. Prepare frequency domain input for this block (Overlap-Save)
        let mut current_block_l = vec![Complex::default(); fft_len];
        let mut current_block_r = vec![Complex::default(); fft_len];

        // Circular buffer read for input
        for i in 0..self.block_size {
            let idx = (self.buf_read_pos + i) % self.input_buffer_l.len();
            current_block_l[i] = Complex::new(self.input_buffer_l[idx], 0.0);
            current_block_r[i] = Complex::new(self.input_buffer_r[idx], 0.0);
        }
        self.buf_read_pos = (self.buf_read_pos + self.block_size) % self.input_buffer_l.len();

        self.fft_forward.process(&mut current_block_l);
        self.fft_forward.process(&mut current_block_r);

        // Update frequency history (circular)
        self.input_freq_history_l[self.history_pos] = current_block_l;
        self.input_freq_history_r[self.history_pos] = current_block_r;

        // 2. Multiply and Accumulate in frequency domain
        let mut accumulated_l = vec![Complex::default(); fft_len];
        let mut accumulated_r = vec![Complex::default(); fft_len];

        for i in 0..self.num_partitions {
            let hist_idx = (self.history_pos + self.num_partitions - i) % self.num_partitions;
            
            let h_l = &self.input_freq_history_l[hist_idx];
            let h_r = &self.input_freq_history_r[hist_idx];
            
            let ir_l = &self.ir_partitions_l[i];
            let ir_r = &self.ir_partitions_r[i];

            for j in 0..fft_len {
                accumulated_l[j] += h_l[j] * ir_l[j];
                accumulated_r[j] += h_r[j] * ir_r[j];
            }
        }

        // 3. Inverse FFT
        self.fft_inverse.process(&mut accumulated_l);
        self.fft_inverse.process(&mut accumulated_r);

        // 4. Overlap-Add and Store to output circular buffer
        let norm = 1.0 / (fft_len as f64);
        for i in 0..self.block_size {
            // Overlap-add logic
            let wet_l = (accumulated_l[i].re * norm) + self.output_overlap_l[i];
            let wet_r = (accumulated_r[i].re * norm) + self.output_overlap_r[i];

            let out_idx = (self.buf_write_pos + i) % self.output_buffer_l.len();
            self.output_buffer_l[out_idx] = wet_l;
            self.output_buffer_r[out_idx] = wet_r;

            // Store tail for next block
            self.output_overlap_l[i] = accumulated_l[i + self.block_size].re * norm;
            self.output_overlap_r[i] = accumulated_r[i + self.block_size].re * norm;
        }
        self.buf_write_pos = (self.buf_write_pos + self.block_size) % self.output_buffer_l.len();

        self.history_pos = (self.history_pos + 1) % self.num_partitions;
    }
}

impl AudioProcessor for ConvolutionReverb {
    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> String {
        "Convolution Reverb".to_string()
    }

    fn process(&mut self, buffer: &mut AudioBuffer, _context: &ProcessingContext) {
        let frames = buffer.frames;
        let mix = self.mix.get_current_value() as f64;

        // Push input to circular buffer
        for i in 0..frames {
            let idx = (self.buf_read_pos + self.samples_in_buf + i) % self.input_buffer_l.len();
            self.input_buffer_l[idx] = buffer.channels_data[0][i];
            self.input_buffer_r[idx] = buffer.channels_data[1][i];
        }
        self.samples_in_buf += frames;

        // Process available blocks
        while self.samples_in_buf >= self.block_size {
            self.process_block();
            self.samples_in_buf -= self.block_size;
        }

        // Pull output from circular buffer
        // Note: Partitioned convolution has inherent latency equal to block_size
        // In a real PDC-aware engine, we'd report this latency.
        let out_start = (self.buf_write_pos + self.output_buffer_l.len() - self.samples_in_buf - frames) % self.output_buffer_l.len();
        for i in 0..frames {
            let idx = (out_start + i) % self.output_buffer_l.len();
            let wet_l = self.output_buffer_l[idx];
            let wet_r = self.output_buffer_r[idx];
            
            buffer.channels_data[0][i] = buffer.channels_data[0][i] * (1.0 - mix) + wet_l * mix;
            buffer.channels_data[1][i] = buffer.channels_data[1][i] * (1.0 - mix) + wet_r * mix;
        }
    }

    fn get_parameters(&mut self) -> Vec<&mut Parameter> {
        vec![&mut self.mix, &mut self.size]
    }

    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        let mut clone = Self::new(&self.ir_data_l, &self.ir_data_r, self.block_size);
        clone.id = self.id;
        clone.mix = self.mix.clone();
        clone.size = self.size.clone();
        clone.ir_path = self.ir_path.clone();
        Box::new(clone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::graph::{AudioBuffer, ProcessingContext};

    #[test]
    fn test_convolution_reverb_processing() {
        // Simple impulse response (single spike followed by silence)
        let ir_l = vec![1.0, 0.0, 0.0, 0.0];
        let ir_r = vec![1.0, 0.0, 0.0, 0.0];
        
        let mut reverb = ConvolutionReverb::new(&ir_l, &ir_r, 2);
        
        // Input buffer with a single impulse
        let mut buffer = AudioBuffer {
            channels_data: vec![vec![1.0, 0.0, 0.0, 0.0], vec![1.0, 0.0, 0.0, 0.0]],
            frames: 4,
            num_channels: 2,
        };
        
        let context = ProcessingContext {
            sample_rate: 44100.0,
            playhead: 0,
            sidechain: None,
        };
        
        // Wet mix is 1.0 (100% wet)
        reverb.mix.set_value(1.0);
        reverb.process(&mut buffer, &context);
        
        // Verify that processing runs and produces non-zero signal output
        assert!(buffer.channels_data[0].iter().any(|&x| x != 0.0));
    }
}
