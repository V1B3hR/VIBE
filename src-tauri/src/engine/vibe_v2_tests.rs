#[cfg(test)]
mod tests {
    use crate::engine::graph::{AudioBuffer, AudioProcessor, ProcessingContext, Track};
    use crate::engine::summing::SummingEngine;
    use crate::engine::vca_group::VcaGroup;
    use crate::engine::convolution_reverb::ConvolutionReverb;
    use crate::engine::multiband_dynamics::MultibandDynamics;
    use crate::engine::library_service::LibraryService;
    use crate::engine::waveform::PyramidCache;
    use std::sync::Arc;
    use uuid::Uuid;

    #[test]
    fn test_vca_gain_scaling() {
        let mut track = Track::new("Test Track".to_string());
        let track_id = track.id;
        
        struct ConstGenerator { val: f64 }
        impl AudioProcessor for ConstGenerator {
            fn process(&mut self, buffer: &mut AudioBuffer, _context: &ProcessingContext) {
                for i in 0..buffer.frames {
                    for c in 0..buffer.num_channels {
                        buffer.channels_data[c][i] = self.val;
                    }
                }
            }
            fn id(&self) -> Uuid { Uuid::new_v4() }
            fn clone_box(&self) -> Box<dyn AudioProcessor> { panic!("Testing only") }
        }

        track.processors.push(Box::new(ConstGenerator { val: 1.0 }));
        
        let mut vca = VcaGroup::new("VCA 1".to_string());
        vca.member_tracks.push(track_id);
        vca.gain.set_value(0.5); // -6dB

        let engine = SummingEngine::new();
        let mut master_l = vec![0.0; 128];
        let mut master_r = vec![0.0; 128];
        let mut master_chans = [master_l.as_mut_slice(), master_r.as_mut_slice()];

        track.output_buffer.frames = 128;
        track.output_buffer.num_channels = 2;

        let fades = Arc::new(crate::engine::fades::FadeLuts::new());
        let hyper_pool = Arc::new(crate::engine::streamer::GlobalBufferPool::new(1));
        let hyper_streamer = Arc::new(crate::engine::streamer::WindowsAsyncStreamer::new(1));

        // Note: process_parallel normally calls track.process which clears the buffer.
        // For this test, we want to see if the summing correctly applies VCA gain.
        // We'll simulate a very simplified process call or just check the math.
        
        engine.process_parallel(
            &mut [track],
            &mut master_chans,
            &[vca],
            44100.0,
            120.0,
            0,
            &fades,
            &[],
            &hyper_pool,
            &hyper_streamer,
            false,
            &vec![vec![0.0; 128]; 64],
            true,
        );

        // Track was 1.0, VCA was 0.5. Pan gain is ~0.707 (center).
        // 1.0 * 0.5 * 0.707 = 0.3535...
        // Master saturation might change it slightly.
        assert!(master_l[0] > 0.3 && master_l[0] < 0.4);
    }

    #[test]
    fn test_convolution_reverb_stereo() {
        let ir_l = vec![1.0, 0.5, 0.25, 0.125];
        let ir_r = vec![0.0, 0.0, 0.0, 1.0]; // Delayed R
        let mut reverb = ConvolutionReverb::new(&ir_l, &ir_r, 4);
        
        let mut buffer = AudioBuffer::new();
        buffer.frames = 4;
        buffer.num_channels = 2;
        buffer.channels_data[0][0] = 1.0; // Impulse L
        buffer.channels_data[1][0] = 1.0; // Impulse R
        
        let context = ProcessingContext {
            sample_rate: 44100.0,
            playhead: 0,
            sidechain: None,
        };
        
        reverb.mix.set_value(1.0); // 100% wet
        reverb.process(&mut buffer, &context);
        
        // Verify output contains the convolved signal in the first block
        assert!(buffer.channels_data[0][0] > 0.0);
    }

    #[test]
    fn test_multiband_dynamics_clone() {
        let mb = MultibandDynamics::new(44100.0);
        let cloned = mb.clone_box();
        assert_eq!(cloned.name(), "Multiband Dynamics");
        assert!(cloned.id() == mb.id());
    }

    #[test]
    fn test_library_tagging_heuristics() {
        let tags = LibraryService::generate_tags("Aggressive_Kick_Dark.wav", &crate::engine::library_service::AudioCategory::Kick);
        assert!(tags.contains(&"kick".to_string()));
        assert!(tags.contains(&"aggressive".to_string()));
    }

    #[test]
    fn test_waveform_streaming_smoke() {
        // Just verify the PyramidCache can be initialized
        let cache = PyramidCache { lods: Vec::new() };
        assert_eq!(cache.lods.len(), 0);
    }

    #[test]
    fn test_disk_writer_init() {
        use crate::engine::disk_writer::DiskWriter;
        let (writer, _prod) = DiskWriter::new(1024);
        writer.stop_recording();
    }
}
