use super::super::graph::{
    AudioBuffer, AudioProcessor, Parameter, ProcessingContext, MAX_BUFFER_SIZE, MAX_CHANNELS,
};
use uuid::Uuid;

/// The "Neural Chain" Smart Slot Wrapper.
/// Wraps any AudioProcessor to provide standard features:
/// Mix (Dry/Wet), Auto-Gain Match, Bypass, and future Oversampling.
pub struct SmartProcessorWrapper {
    pub inner: Box<dyn AudioProcessor>,
    pub id: Uuid,
    pub mix: Parameter,
    pub output_gain: Parameter,
    pub is_bypassed: bool,
    pub is_soloed: bool,
    pub auto_gain_enabled: bool,
    pub oversampling_factor: u32,

    // Internal state
    dry_buffer: [[f64; MAX_BUFFER_SIZE]; MAX_CHANNELS],
    gain_matching_factor: f64,
}

impl SmartProcessorWrapper {
    pub fn new(inner: Box<dyn AudioProcessor>) -> Self {
        Self {
            id: inner.id(),
            inner,
            mix: Parameter::new("Mix", 1.0, 0.0, 1.0),
            output_gain: Parameter::new("Gain", 1.0, 0.0, 4.0),
            is_bypassed: false,
            is_soloed: false,
            auto_gain_enabled: false,
            oversampling_factor: 1,
            dry_buffer: [[0.0; MAX_BUFFER_SIZE]; MAX_CHANNELS],
            gain_matching_factor: 1.0,
        }
    }
}

impl AudioProcessor for SmartProcessorWrapper {
    fn process(&mut self, buffer: &mut AudioBuffer, context: &ProcessingContext) {
        let playhead = context.playhead;
        if self.is_bypassed {
            return;
        }

        let frames = buffer.frames;
        let channels = buffer.num_channels;

        // 1. Capture Dry Signal for Mix and Auto-Gain Reference
        for c in 0..channels {
            self.dry_buffer[c][..frames].copy_from_slice(&buffer.channels_data[c][..frames]);
        }

        // 2. Process Inner Plugin
        self.inner.process(buffer, context);

        // 3. Auto-Gain Matching (Loudness Normalization)
        if self.auto_gain_enabled {
            let mut dry_energy = 0.0;
            let mut wet_energy = 0.0;

            for c in 0..channels {
                for i in 0..frames {
                    dry_energy += self.dry_buffer[c][i] * self.dry_buffer[c][i];
                    wet_energy += buffer.channels_data[c][i] * buffer.channels_data[c][i];
                }
            }

            let dry_rms = (dry_energy / (frames * channels) as f64).sqrt();
            let wet_rms = (wet_energy / (frames * channels) as f64).sqrt();

            if wet_rms > 1e-7 {
                let target_factor = dry_rms / wet_rms;
                // Simple smoothing to prevent rapid volume pumping
                self.gain_matching_factor = self.gain_matching_factor * 0.9 + target_factor * 0.1;

                for c in 0..channels {
                    for i in 0..frames {
                        buffer.channels_data[c][i] *= self.gain_matching_factor;
                    }
                }
            }
        }

        // 4. Dry/Wet Mix
        let mix = self.mix.get_value_at(playhead);
        if mix < 0.999 {
            for c in 0..channels {
                for i in 0..frames {
                    let dry = self.dry_buffer[c][i];
                    let wet = buffer.channels_data[c][i];
                    buffer.channels_data[c][i] = dry * (1.0 - mix) + wet * mix;
                }
            }
        }

        // 5. Global Output Gain
        let gain = self.output_gain.get_value_at(playhead);
        if (gain - 1.0).abs() > 1e-6 {
            for c in 0..channels {
                for i in 0..frames {
                    buffer.channels_data[c][i] *= gain;
                }
            }
        }
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        Box::new(Self {
            inner: self.inner.clone_box(),
            id: self.id,
            mix: self.mix.clone(),
            output_gain: self.output_gain.clone(),
            is_bypassed: self.is_bypassed,
            is_soloed: self.is_soloed,
            auto_gain_enabled: self.auto_gain_enabled,
            oversampling_factor: self.oversampling_factor,
            dry_buffer: [[0.0; MAX_BUFFER_SIZE]; MAX_CHANNELS],
            gain_matching_factor: self.gain_matching_factor,
        })
    }

    fn name(&self) -> String {
        format!("{} (V1B3)", self.inner.name())
    }

    fn on_midi_event(&mut self, status: u8, data1: u16, data2: u32) {
        self.inner.on_midi_event(status, data1, data2);
    }

    fn get_parameters(&mut self) -> Vec<&mut Parameter> {
        let mut params = vec![&mut self.mix, &mut self.output_gain];
        params.extend(self.inner.get_parameters());
        params
    }

    fn latency_samples(&self) -> usize {
        self.inner.latency_samples()
    }

    fn get_state(&self) -> Vec<u8> {
        self.inner.get_state()
    }

    fn set_state(&mut self, state: &[u8]) {
        self.inner.set_state(state)
    }

    fn open_editor(&mut self, handle: *mut std::ffi::c_void) -> Option<(u32, u32)> {
        self.inner.open_editor(handle)
    }

    fn close_editor(&mut self) {
        self.inner.close_editor();
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self.inner.as_any()
    }

    fn set_bypass(&mut self, bypass: bool) {
        self.is_bypassed = bypass;
    }

    fn is_bypassed(&self) -> bool {
        self.is_bypassed
    }

    fn drain_plugin_feedback(&self) -> Vec<(String, f64)> {
        self.inner.drain_plugin_feedback()
    }

    fn get_cpu_usage(&self) -> f32 {
        self.inner.get_cpu_usage()
    }

    fn get_programs(&self) -> Vec<String> {
        self.inner.get_programs()
    }

    fn set_program(&mut self, index: i32) {
        self.inner.set_program(index);
    }

    fn needs_pdc_recalc(&self) -> bool {
        self.inner.needs_pdc_recalc()
    }

    fn reset_pdc_recalc(&mut self) {
        self.inner.reset_pdc_recalc();
    }

    fn poll_editor_resize(&self) -> Option<(u32, u32)> {
        self.inner.poll_editor_resize()
    }
}
