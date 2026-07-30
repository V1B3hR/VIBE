pub struct RmsTracker {
    rms: f64,
    smoothing_factor: f64, // 0.0 to 1.0 (higher = slower)
}

impl RmsTracker {
    pub fn new(smoothing_factor: f64) -> Self {
        Self {
            rms: 0.0,
            smoothing_factor,
        }
    }

    pub fn process(&mut self, sample: f64) {
        let squared = sample * sample;
        // Exponential smoothing
        self.rms = self.smoothing_factor * self.rms + (1.0 - self.smoothing_factor) * squared;
    }

    pub fn get_rms(&self) -> f64 {
        self.rms.sqrt().max(1e-12)
    }

    pub fn get_db(&self) -> f64 {
        20.0 * self.get_rms().log10()
    }
}

pub struct AutoGain {
    input_rms: RmsTracker,
    output_rms: RmsTracker,
    current_gain: f64,
    enabled: bool,
}

impl AutoGain {
    pub fn new(smoothing: f64) -> Self {
        Self {
            input_rms: RmsTracker::new(smoothing),
            output_rms: RmsTracker::new(smoothing),
            current_gain: 1.0,
            enabled: false,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.current_gain = 1.0;
        }
    }

    pub fn process(&mut self, input: f64, output: f64) -> f64 {
        if !self.enabled {
            return output;
        }

        self.input_rms.process(input);
        self.output_rms.process(output);

        let in_rms = self.input_rms.get_rms();
        let out_rms = self.output_rms.get_rms();

        // Calculate target gain to match input volume
        let target_gain = in_rms / out_rms;

        // Very slow smoothing for the gain itself to avoid artifacts
        self.current_gain = 0.999 * self.current_gain + 0.001 * target_gain;

        output * self.current_gain
    }
}
