#[cfg(test)]
mod tests {
    use crate::engine::graph::*;

    #[test]
    fn test_denormal_protection() {
        // Test that very small numbers are flushed to zero
        assert_eq!(flush_denormal_f64(1e-16), 0.0);
        assert_eq!(flush_denormal_f64(-1e-16), 0.0);

        // Test that normal numbers pass through
        assert_eq!(flush_denormal_f64(0.5), 0.5);
        assert_eq!(flush_denormal_f64(-0.5), -0.5);
    }

    #[test]
    fn test_audio_buffer_creation() {
        let buffer = AudioBuffer::new();
        assert_eq!(buffer.frames, 0);
        assert_eq!(buffer.num_channels, 0);
    }

    #[test]
    fn test_audio_buffer_clear() {
        let mut buffer = AudioBuffer::new();
        buffer.frames = 10;
        buffer.num_channels = 2;

        // Fill with non-zero values
        for i in 0..10 {
            buffer.channels_data[0][i] = 1.0;
            buffer.channels_data[1][i] = 1.0;
        }

        buffer.clear();

        // Verify all cleared
        for i in 0..10 {
            assert_eq!(buffer.channels_data[0][i], 0.0);
            assert_eq!(buffer.channels_data[1][i], 0.0);
        }
    }

    #[test]
    fn test_parameter_atomic_operations() {
        let param = Parameter::new("Test", 0.5, 0.0, 1.0);

        // Test initial value
        assert_eq!(param.get_current_value(), 0.5);

        // Test set/get
        param.set_value(0.75);
        assert_eq!(param.get_current_value(), 0.75);
    }

    #[test]
    fn test_gain_effect_unity() {
        let mut gain = GainEffect::new(1.0);
        let mut buffer = AudioBuffer::new();
        buffer.frames = 10;
        buffer.num_channels = 2;

        // Fill with test signal
        for i in 0..10 {
            buffer.channels_data[0][i] = 0.5;
            buffer.channels_data[1][i] = 0.5;
        }

        let context = ProcessingContext {
            sample_rate: 44100.0,
            playhead: 0,
            sidechain: None,
        };
        gain.process(&mut buffer, &context);

        // Unity gain should not change signal
        for i in 0..10 {
            assert_eq!(buffer.channels_data[0][i], 0.5);
            assert_eq!(buffer.channels_data[1][i], 0.5);
        }
    }

    #[test]
    fn test_gain_effect_amplification() {
        let mut gain = GainEffect::new(2.0);
        let mut buffer = AudioBuffer::new();
        buffer.frames = 10;
        buffer.num_channels = 2;

        // Fill with test signal
        for i in 0..10 {
            buffer.channels_data[0][i] = 0.5;
            buffer.channels_data[1][i] = 0.5;
        }

        let context = ProcessingContext {
            sample_rate: 44100.0,
            playhead: 0,
            sidechain: None,
        };
        gain.process(&mut buffer, &context);

        // 2x gain should double signal
        for i in 0..10 {
            assert_eq!(buffer.channels_data[0][i], 1.0);
            assert_eq!(buffer.channels_data[1][i], 1.0);
        }
    }

    #[test]
    fn test_lowpass_filter_dc_offset() {
        let mut filter = LowPassFilter::new(1.0); // Full cutoff = pass through
        let mut buffer = AudioBuffer::new();
        buffer.frames = 100;
        buffer.num_channels = 2;

        // DC offset test (constant signal)
        for i in 0..100 {
            buffer.channels_data[0][i] = 1.0;
            buffer.channels_data[1][i] = 1.0;
        }

        let context = ProcessingContext {
            sample_rate: 44100.0,
            playhead: 0,
            sidechain: None,
        };
        filter.process(&mut buffer, &context);

        // After settling, DC should pass through
        for i in 50..100 {
            assert!((buffer.channels_data[0][i] - 1.0).abs() < 0.01);
            assert!((buffer.channels_data[1][i] - 1.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_track_creation() {
        let track = Track::new("Test Track".to_string());
        assert_eq!(track.name, "Test Track");
        assert_eq!(track.is_muted, false);
        assert_eq!(track.is_solo, false);
        assert_eq!(track.output_buffer.channels_data[0].len(), MAX_BUFFER_SIZE);
        assert_eq!(track.output_buffer.channels_data[1].len(), MAX_BUFFER_SIZE);
    }

    #[test]
    fn test_bus_clone() {
        let bus = Bus::new("TestBus".to_string(), "#FF0000".to_string());
        bus.volume.set_value(0.5);
        let cloned_bus = bus.clone();

        assert_eq!(cloned_bus.name, "TestBus");
        assert_eq!(cloned_bus.volume.get_current_value(), 0.5);
        // IDs should match for synchronization
        assert_eq!(cloned_bus.id, bus.id);
    }
}
