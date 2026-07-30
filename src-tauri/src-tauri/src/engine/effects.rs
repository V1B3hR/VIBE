
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
        hp.process(&mut buffer, 44100.0, 0);
        // High pass of DC step should decay
        assert!(buffer.channels_data[0][99].abs() < 1.0);
    }
}
