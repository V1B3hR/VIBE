use super::graph::{AudioBuffer, AudioProcessor, Parameter, ProcessingContext};
use uuid::Uuid;

pub struct StereoImager {
    id: Uuid,
    pub width: Parameter,
    pub mid_gain: Parameter,
    pub side_gain: Parameter,
    pub crossover_freq: Parameter, // Multiband imager?

                                   // Internal state for multiband if needed
}

impl StereoImager {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            width: Parameter::new("Width", 1.0, 0.0, 2.0),
            mid_gain: Parameter::new("Mid Gain", 0.0, -12.0, 12.0),
            side_gain: Parameter::new("Side Gain", 0.0, -12.0, 12.0),
            crossover_freq: Parameter::new("Xover", 200.0, 20.0, 1000.0),
        }
    }
}

impl AudioProcessor for StereoImager {
    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> String {
        "Stereo Imager".to_string()
    }

    fn process(&mut self, buffer: &mut AudioBuffer, _context: &ProcessingContext) {
        let frames = buffer.frames;
        let width = self.width.get_current_value();
        let m_gain = 10.0f64.powf(self.mid_gain.get_current_value() / 20.0);
        let s_gain = 10.0f64.powf(self.side_gain.get_current_value() / 20.0);

        // Simpler implementation first: Global M/S
        for i in 0..frames {
            let l = buffer.channels_data[0][i];
            let r = buffer.channels_data[1][i];

            // Encode
            let mut mid = (l + r) * 0.5;
            let mut side = (l - r) * 0.5;

            // Process
            mid *= m_gain;
            side *= s_gain * width;

            // Decode
            buffer.channels_data[0][i] = mid + side;
            buffer.channels_data[1][i] = mid - side;
        }
    }

    fn get_parameters(&mut self) -> Vec<&mut Parameter> {
        vec![
            &mut self.width,
            &mut self.mid_gain,
            &mut self.side_gain,
            &mut self.crossover_freq,
        ]
    }

    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        Box::new(super::graph::DummyProcessor {
            id: self.id,
            name: self.name(),
            parameters: Vec::new(),
        })
    }
}
