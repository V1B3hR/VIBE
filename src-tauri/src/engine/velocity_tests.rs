#[cfg(test)]
mod tests {
    use super::super::velocity::*;
    use std::fs::File;
    use std::io::Write;
    use std::sync::Arc;

    // Helper to create a dummy file for SmartClip (even if we don't read it in this specific test)
    fn create_dummy_file(path: &str, size: usize) {
        let mut f = File::create(path).unwrap();
        let data = vec![0u8; size];
        f.write_all(&data).unwrap();
    }

    #[test]
    fn test_smart_clip_creation() {
        let path = "test_clip_creation.bin";
        create_dummy_file(path, 1024);

        let head = vec![0.1, 0.2, 0.3];
        let clip = SmartClip::new(path, head.clone(), 1000).expect("Failed to create SmartClip");

        assert_eq!(clip.sample_rate, 44100);
        assert_eq!(clip.channels, 2);
        assert_eq!(clip.head_buffer.len(), 3);

        // Cleanup
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_streaming_voice_head_to_tail() {
        let path = "test_streaming.bin";
        create_dummy_file(path, 1024);

        // Head has 3 samples
        let head = vec![1.0, 2.0, 3.0];
        let clip = Arc::new(SmartClip::new(path, head, 100).unwrap());

        // Create voice with small buffer
        let (mut voice, mut producer) = StreamingVoice::new(clip.clone(), 16);

        // 1. Read Head (samples 0, 1, 2)
        assert_eq!(voice.get_next_sample(), 1.0);
        assert_eq!(voice.get_next_sample(), 2.0);
        assert_eq!(voice.get_next_sample(), 3.0);

        // 2. Next sample (3) should come from consumer (RingBuffer)
        // Simulate Disk Engine pushing data
        producer.push(4.0).unwrap();
        producer.push(5.0).unwrap();

        assert_eq!(voice.get_next_sample(), 4.0);
        assert_eq!(voice.get_next_sample(), 5.0);

        // 3. Verify underflow handling (defaults to 0.0)
        assert_eq!(voice.get_next_sample(), 0.0);

        let _ = std::fs::remove_file(path);
    }
}
