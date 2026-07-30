use super::audio_commands::{GraphCommand, MidiEvent, ParamChange};
use crate::engine::audio_preview::PreviewVoice;
use crate::engine::graph::{AudioBuffer, AudioProcessor, Bus, Track};
use crate::engine::vca_group::VcaGroup;
use std::time::Duration;

pub fn get_micros() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_micros() as u64
}

#[allow(dead_code)]
pub struct DspState {
    pub internal_tracks: Vec<Track>,
    pub internal_busses: Vec<Bus>,
    pub internal_vca_groups: Vec<VcaGroup>,
    pub internal_engine_fx: Option<Box<dyn AudioProcessor>>,
    pub internal_master_limiter: Option<Box<dyn AudioProcessor>>,
    pub master_buffer: AudioBuffer,
    pub master_channels: Vec<Vec<f64>>, // Support for multichannel (e.g. 7.1.4)
    pub num_master_channels: usize,
    #[allow(dead_code)]
    pub monitor_input_buffer: Vec<f32>,
    pub pdc_dirty: bool,
    pub preview_voice: Option<PreviewVoice>,
    // Issue #6 Fix: Pre-allocated hardware input buffers (64 channels x 4096 samples)
    #[allow(dead_code)] // Used in audio callback
    pub hardware_inputs: Vec<Vec<f32>>,

    // Phase 4: Interaction & Expression
    pub mpe_handler: super::mpe_handler::MpeHandler,
    pub macro_engine: super::macro_engine::MacroEngine,
    pub clip_launcher: super::clip_launcher::ClipLauncher,
}

impl DspState {
    pub fn set_parameter(&mut self, param_id: uuid::Uuid, value: f64) {
        for track in self.internal_tracks.iter_mut() {
            if track.volume.id == param_id {
                track.volume.set_value(value);
                return;
            }
            if track.pan.id == param_id {
                track.pan.set_value(value);
                return;
            }
            for proc in track.processors.iter_mut() {
                for param in proc.get_parameters() {
                    if param.id == param_id {
                        param.set_value(value);
                        return;
                    }
                }
            }
        }

        for vca in self.internal_vca_groups.iter_mut() {
            if vca.gain.id == param_id {
                vca.gain.set_value(value);
                return;
            }
        }
    }
}

unsafe impl Send for DspState {}

pub struct StreamConsumers {
    pub midi_cons: rtrb::Consumer<MidiEvent>,
    pub param_cons: rtrb::Consumer<ParamChange>,
    pub graph_cons: rtrb::Consumer<GraphCommand>,
    pub hardware_input_cons: Vec<rtrb::Consumer<f32>>,
}
