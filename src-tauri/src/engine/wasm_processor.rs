#![allow(dead_code)]
use super::graph::{AudioBuffer, AudioProcessor, Parameter, ProcessingContext};
use super::wasm::WasmPlugin;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Wrapper that makes a WASM plugin compatible with VIBE's AudioProcessor trait
pub struct WasmAudioProcessor {
    plugin: WasmPlugin,
    id: Uuid,
    name: String,
    parameters: Vec<Parameter>,
    // Store original WASM bytes to allow re-instantiation (Cloning)
    wasm_bytes: Vec<u8>,
    // Pre-allocated buffers for real-time safety (Avoid Vec in process())
    internal_output_l: Vec<f64>,
    internal_output_r: Vec<f64>,
}

#[derive(Serialize, Deserialize)]
struct WasmState {
    parameters: Vec<(String, f64)>,
    // Future: Internal WASM memory blob if needed
}

impl WasmAudioProcessor {
    pub fn new(wasm_bytes: &[u8], name: String, sample_rate: f64) -> Result<Self, String> {
        let plugin = WasmPlugin::new(wasm_bytes, sample_rate, 512)?;

        // TODO: Query plugin for actual parameters
        // For now, we inject some "Generic" parameters to prove UI works
        let mut parameters = Vec::new();
        parameters.push(Parameter::new("Dry/Wet", 1.0, 0.0, 1.0));
        parameters.push(Parameter::new("Gain", 0.8, 0.0, 2.0));

        Ok(WasmAudioProcessor {
            plugin,
            id: Uuid::new_v4(),
            name,
            parameters,
            wasm_bytes: wasm_bytes.to_vec(),
            internal_output_l: vec![0.0; 8192], // Large enough for most buffers
            internal_output_r: vec![0.0; 8192],
        })
    }

    pub fn add_parameter(&mut self, name: String, min: f64, max: f64, default: f64) {
        self.parameters
            .push(Parameter::new(&name, default, min, max));
    }
}

impl AudioProcessor for WasmAudioProcessor {
    fn process(&mut self, buffer: &mut AudioBuffer, _context: &ProcessingContext) {
        let frames = buffer.frames;

        // Ensure internal buffers are large enough
        if self.internal_output_l.len() < frames {
            self.internal_output_l.resize(frames, 0.0);
            self.internal_output_r.resize(frames, 0.0);
        }

        // Sync parameters to WASM if changed (Naively for now)
        for (idx, param) in self.parameters.iter().enumerate() {
            let _ = self.plugin.set_parameter(idx as u32, param.value);
        }

        // Extract planar data
        let input_l = &buffer.channels_data[0][..frames];
        let input_r = &buffer.channels_data[1][..frames];

        // Process through WASM (isolated, safe)
        if let Err(e) = self.plugin.process(
            input_l,
            input_r,
            &mut self.internal_output_l[..frames],
            &mut self.internal_output_r[..frames],
        ) {
            eprintln!("WASM Plugin Error: {}", e);
            // On error, just pass through (safety)
            return;
        }

        // Write back to buffer
        buffer.channels_data[0][..frames].copy_from_slice(&self.internal_output_l[..frames]);
        buffer.channels_data[1][..frames].copy_from_slice(&self.internal_output_r[..frames]);
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn clone_box(&self) -> Box<dyn AudioProcessor> {
        // Re-instantiate the plugin from source bytes
        // This ensures the clone is a fresh, valid instance, not a Dummy
        // We use 44100.0 as default SR, but ideally this should be passed or stored
        match WasmAudioProcessor::new(&self.wasm_bytes, self.name.clone(), 44100.0) {
            Ok(mut processor) => {
                processor.id = self.id; // Keep ID
                processor.parameters = self.parameters.clone();
                // Sync all parameters to the WASM instance
                for (idx, param) in processor.parameters.iter().enumerate() {
                    let _ = processor.plugin.set_parameter(idx as u32, param.value);
                }
                Box::new(processor)
            }
            Err(e) => {
                eprintln!("Failed to clone WASM Processor: {}", e);
                // Fallback to Dummy if re-creation fails (shouldn't happen if bytes are valid)
                Box::new(super::graph::DummyProcessor {
                    id: self.id,
                    name: format!("{} (Dead)", self.name),
                    parameters: self.parameters.clone(),
                })
            }
        }
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn get_parameters(&mut self) -> Vec<&mut Parameter> {
        self.parameters.iter_mut().collect()
    }

    fn get_state(&self) -> Vec<u8> {
        // Serialize current parameter values
        let params_simple: Vec<(String, f64)> = self
            .parameters
            .iter()
            .map(|p| (p.name.clone(), p.value))
            .collect();

        let state = WasmState {
            parameters: params_simple,
        };
        bincode::serialize(&state).unwrap_or_default()
    }

    fn set_state(&mut self, state_data: &[u8]) {
        if let Ok(state) = bincode::deserialize::<WasmState>(state_data) {
            for (name, val) in state.parameters {
                if let Some(param) = self.parameters.iter_mut().find(|p| p.name == name) {
                    param.value = val;
                }
            }
            // Sync to WASM
            let _ = self.plugin.set_parameter(1, self.parameters[1].value);
        }
    }

    fn on_midi_event(&mut self, _status: u8, _data1: u16, _data2: u32) {
        // WASM plugins can optionally handle high-res MIDI
    }
}
