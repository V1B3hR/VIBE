/// State for the preview voice
#[derive(Clone)]
pub struct PreviewVoice {
    pub data: Vec<f32>,
    pub position: usize,
    #[allow(dead_code)]
    pub start_sample: u64,
    pub volume: f32,
    pub is_playing: bool,
}

#[allow(dead_code)]
impl PreviewVoice {
    pub fn new(data: Vec<f32>, start_sample: u64) -> Self {
        Self {
            data,
            position: 0,
            start_sample,
            volume: 0.8,
            is_playing: true,
        }
    }

    pub fn process(&mut self, output: &mut [f32], channels: usize) {
        if !self.is_playing {
            return;
        }

        for frame in output.chunks_mut(channels) {
            if self.position < self.data.len() {
                let sample_l = self.data[self.position];
                let sample_r = if self.position + 1 < self.data.len() {
                    self.data[self.position + 1]
                } else {
                    sample_l
                };

                // Mix into output (assuming output is already initialized/mixed)
                if channels >= 2 {
                    frame[0] += sample_l * self.volume;
                    frame[1] += sample_r * self.volume;
                } else {
                    frame[0] += (sample_l + sample_r) * 0.5 * self.volume;
                }

                self.position += 2;
            } else {
                self.is_playing = false;
                break;
            }
        }
    }
}
