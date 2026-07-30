use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum MidiResolution {
    SevenBit, // Standard CC (0-127)
    FourteenBit {
        // High-res (CC paired: 0-31 MSB, 32-63 LSB)
        msb_cc: u8,
        lsb_cc: u8,
    },
    Nrpn {
        // Non-Registered Parameter Number
        msb: u8,
        lsb: u8,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum KnobMode {
    Absolute,          // 0-127 mapped to min-max
    Relative,          // Signed delta (e.g. +1 / -1 or two's complement)
    RelativeBinOffset, // 64 = 0, <64 dec, >64 inc
    Toggle,            // Button: 0->1 transition toggles bool
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MidiBinding {
    pub id: Uuid,

    // Hardware Source
    pub device_hash: u64, // xxhash of device name
    pub channel: u8,      // 0-15
    pub cc_number: u8,    // Primary CC
    pub resolution: MidiResolution,

    // Software Targets (Multi-parameter support)
    pub targets: Vec<ParameterTarget>,

    // Logic
    pub mode: KnobMode,
    pub bidirectional: bool, // Send feedback to hardware

    #[serde(skip, default = "default_takeover_state")]
    pub takeover_state: Arc<SoftTakeoverState>,
}

fn default_takeover_state() -> Arc<SoftTakeoverState> {
    Arc::new(SoftTakeoverState::default())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParameterTarget {
    pub param_id: Uuid,
    pub min: f64,
    pub max: f64,
    pub scale: f64, // Sensitivity/Ratio
    pub invert: bool,
}

#[derive(Debug)]
pub struct SoftTakeoverState {
    pub engaged: AtomicBool,
    pub last_hw_value: AtomicU8,
}

impl Default for SoftTakeoverState {
    fn default() -> Self {
        Self {
            engaged: AtomicBool::new(false),
            last_hw_value: AtomicU8::new(0),
        }
    }
}

impl Default for MidiBinding {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            device_hash: 0,
            channel: 0,
            cc_number: 0,
            resolution: MidiResolution::SevenBit,
            targets: Vec::new(),
            mode: KnobMode::Absolute,
            bidirectional: false,
            takeover_state: Arc::new(SoftTakeoverState::default()),
        }
    }
}

use arc_swap::ArcSwap;
use std::collections::HashMap;

pub struct NeuralMapper {
    // Fast lookup for incoming MIDI events
    // Key: (device_hash, channel, cc)
    pub bindings: ArcSwap<HashMap<(u64, u8, u8), MidiBinding>>,

    pub is_learning: AtomicBool,
    pub learning_target: Mutex<Option<Uuid>>,

    // Phase 4: High-res state tracking (CC values + NRPN state)
    pub hi_res_state: Mutex<HashMap<(u64, u8), HiResState>>,
}

pub struct HiResState {
    pub cc_vals: [u8; 128],
    pub nrpn_msb: u8,
    pub nrpn_lsb: u8,
}

impl Default for HiResState {
    fn default() -> Self {
        Self {
            cc_vals: [0u8; 128],
            nrpn_msb: 0,
            nrpn_lsb: 0,
        }
    }
}

impl NeuralMapper {
    pub fn new() -> Self {
        Self {
            bindings: ArcSwap::from_pointee(HashMap::new()),
            is_learning: AtomicBool::new(false),
            learning_target: Mutex::new(None),
            hi_res_state: Mutex::new(HashMap::new()),
        }
    }
}

#[derive(Debug)]
pub enum MappingResult {
    None,
    ParameterUpdates(Vec<(Uuid, f64)>),
    BindingLearned(MidiBinding),
}

impl NeuralMapper {
    pub fn process_cc<F>(
        &self,
        device_hash: u64,
        channel: u8,
        cc: u8,
        value: u8,
        mut value_provider: F,
    ) -> MappingResult
    where
        F: FnMut(Uuid) -> f64,
    {
        // 0. Update high-res state
        let mut hr_guard = self.hi_res_state.lock().unwrap();
        let hr_state = hr_guard.entry((device_hash, channel)).or_default();

        hr_state.cc_vals[cc as usize] = value;
        if cc == 99 {
            hr_state.nrpn_msb = value;
        }
        if cc == 98 {
            hr_state.nrpn_lsb = value;
        }

        // 1. Learning Mode Check
        if self.is_learning.load(Ordering::Acquire) {
            let target = self.learning_target.lock().unwrap();
            if let Some(param_id) = *target {
                // Detect Movement (Simple threshold or just any movement for MVP)
                // For a robust implementation, we might wait for a few messages or a range.
                // For now: First event binds it.

                let binding = MidiBinding {
                    id: Uuid::new_v4(),
                    device_hash,
                    channel,
                    cc_number: cc,
                    resolution: MidiResolution::SevenBit,
                    targets: vec![ParameterTarget {
                        param_id,
                        min: 0.0, // Default assume full range
                        max: 1.0,
                        scale: 1.0,
                        invert: false,
                    }],
                    mode: KnobMode::Absolute, // Default
                    bidirectional: false,
                    takeover_state: Arc::new(SoftTakeoverState::default()),
                };

                return MappingResult::BindingLearned(binding);
            }
        }

        // 2. Lookup Binding
        let map = self.bindings.load();

        // High-res lookup: If this CC is an LSB or NRPN Data Entry, we might still match a binding indexed by MSB
        let binding = if let Some(b) = map.get(&(device_hash, channel, cc)) {
            Some(b)
        } else {
            // Scan for 14-bit LSB or NRPN bindings
            map.values().find(|b| {
                b.device_hash == device_hash
                    && b.channel == channel
                    && match b.resolution {
                        MidiResolution::FourteenBit { lsb_cc, .. } => lsb_cc == cc,
                        MidiResolution::Nrpn { .. } => cc == 6 || cc == 38,
                        _ => false,
                    }
            })
        };

        if let Some(binding) = binding {
            let mut updates = Vec::new();

            // 3. Resolve HW Value based on Resolution
            let (hw_normalized, bit_depth_max) = match binding.resolution {
                MidiResolution::SevenBit => (value as f64 / 127.0, 127.0),
                MidiResolution::FourteenBit { msb_cc, lsb_cc } => {
                    let msb = hr_state.cc_vals[msb_cc as usize] as u16;
                    let lsb = hr_state.cc_vals[lsb_cc as usize] as u16;
                    let combined = (msb << 7) | lsb;
                    (combined as f64 / 16383.0, 16383.0)
                }
                MidiResolution::Nrpn { msb, lsb } => {
                    if hr_state.nrpn_msb == msb && hr_state.nrpn_lsb == lsb {
                        let d_msb = hr_state.cc_vals[6] as u16;
                        let d_lsb = hr_state.cc_vals[38] as u16;
                        let combined = (d_msb << 7) | d_lsb;
                        (combined as f64 / 16383.0, 16383.0)
                    } else {
                        return MappingResult::None;
                    }
                }
            };

            match binding.mode {
                KnobMode::Absolute => {
                    let state = &binding.takeover_state;
                    let mut is_engaged = state.engaged.load(Ordering::Relaxed);
                    let last_hw = state.last_hw_value.load(Ordering::Relaxed);

                    // Check Soft Takeover
                    if let Some(first_target) = binding.targets.first() {
                        if !is_engaged {
                            let current_val = value_provider(first_target.param_id);
                            let diff = (hw_normalized - current_val).abs();
                            if diff < 0.05 {
                                is_engaged = true;
                            } else {
                                let last_hw_norm = last_hw as f64 / bit_depth_max;
                                if (hw_normalized > current_val && last_hw_norm < current_val)
                                    || (hw_normalized < current_val && last_hw_norm > current_val)
                                {
                                    is_engaged = true;
                                }
                            }
                        }
                    } else {
                        is_engaged = true;
                    }

                    // Update State
                    if is_engaged != state.engaged.load(Ordering::Relaxed) {
                        state.engaged.store(is_engaged, Ordering::Relaxed);
                    }
                    state.last_hw_value.store(value, Ordering::Relaxed);

                    if is_engaged {
                        for target in &binding.targets {
                            let mut mapped = hw_normalized;
                            if target.invert {
                                mapped = 1.0 - mapped;
                            }
                            mapped = target.min + mapped * (target.max - target.min);
                            updates.push((target.param_id, mapped));
                        }
                    }
                }
                KnobMode::Relative | KnobMode::RelativeBinOffset => {
                    let delta = if binding.mode == KnobMode::RelativeBinOffset {
                        (value as f64) - 64.0
                    } else {
                        // Standard "Relative 2s Complement" approx: 1..63 = +, 127..65 = -
                        if value <= 63 {
                            value as f64
                        } else {
                            (value as f64) - 128.0
                        }
                    };

                    for target in &binding.targets {
                        let current_norm = value_provider(target.param_id);
                        let sensitivity = 0.02 * target.scale;
                        // Invert delta if target is inverted?
                        let d = if target.invert { -delta } else { delta };
                        let new_norm = (current_norm + d * sensitivity).clamp(0.0, 1.0);

                        let mapped = target.min + new_norm * (target.max - target.min);
                        updates.push((target.param_id, mapped));
                    }
                }
                KnobMode::Toggle => {
                    if value > 64 {
                        // Button Press
                        for target in &binding.targets {
                            let current_norm = value_provider(target.param_id);
                            // Toggle: if > 0.5 then 0.0 else 1.0
                            let new_norm = if current_norm > 0.5 { 0.0 } else { 1.0 };
                            let mapped = target.min + new_norm * (target.max - target.min);
                            updates.push((target.param_id, mapped));
                        }
                    }
                }
            }

            return MappingResult::ParameterUpdates(updates);
        }

        MappingResult::None
    }

    // Call this from Main Thread to add binding
    pub fn add_binding(&self, binding: MidiBinding) {
        let mut new_map = (**self.bindings.load()).clone();
        new_map.insert(
            (binding.device_hash, binding.channel, binding.cc_number),
            binding,
        );
        self.bindings.store(Arc::new(new_map));
    }

    pub fn remove_binding(&self, id: Uuid) {
        let mut new_map = (**self.bindings.load()).clone();
        new_map.retain(|_, v| v.id != id);
        self.bindings.store(Arc::new(new_map));
    }

    pub fn get_bindings(&self) -> Vec<MidiBinding> {
        let map = self.bindings.load();
        map.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_learning() {
        let mapper = NeuralMapper::new();
        let param_id = Uuid::new_v4();

        // Enable learning
        mapper.is_learning.store(true, Ordering::Release);
        *mapper.learning_target.lock().unwrap() = Some(param_id);

        let result = mapper.process_cc(1234, 0, 10, 64, |_| 0.0);

        if let MappingResult::BindingLearned(binding) = result {
            assert_eq!(binding.cc_number, 10);
            assert_eq!(binding.device_hash, 1234);
            assert_eq!(binding.targets[0].param_id, param_id);
        } else {
            panic!("Expected BindingLearned");
        }
    }

    #[test]
    fn test_absolute_soft_takeover() {
        let mapper = NeuralMapper::new();
        let param_id = Uuid::new_v4();

        let binding = MidiBinding {
            id: Uuid::new_v4(),
            device_hash: 1,
            channel: 0,
            cc_number: 10,
            resolution: MidiResolution::SevenBit,
            targets: vec![ParameterTarget {
                param_id,
                min: 0.0,
                max: 1.0,
                scale: 1.0,
                invert: false,
            }],
            mode: KnobMode::Absolute,
            bidirectional: false,
            takeover_state: Arc::new(SoftTakeoverState::default()),
        };
        mapper.add_binding(binding);

        // 1. Send CC far from current value (0.5)
        // HW = 0.1, Current = 0.5 -> Should not engage
        let result = mapper.process_cc(1, 0, 10, 12, |_| 0.5);
        if let MappingResult::ParameterUpdates(updates) = result {
            assert!(updates.is_empty(), "Should not update before takeover");
        }

        // 2. Send CC close to current value (0.48)
        // HW = 0.48, Current = 0.5 -> Should engage
        let result = mapper.process_cc(1, 0, 10, 61, |_| 0.5);
        if let MappingResult::ParameterUpdates(updates) = result {
            assert_eq!(updates.len(), 1);
            assert!((updates[0].1 - 0.4803).abs() < 0.001);
        } else {
            panic!("Expected ParameterUpdates");
        }
    }

    #[test]
    fn test_relative_mapping() {
        let mapper = NeuralMapper::new();
        let param_id = Uuid::new_v4();

        let binding = MidiBinding {
            id: Uuid::new_v4(),
            device_hash: 1,
            channel: 0,
            cc_number: 10,
            resolution: MidiResolution::SevenBit,
            targets: vec![ParameterTarget {
                param_id,
                min: 0.0,
                max: 100.0, // Test mapped range
                scale: 1.0,
                invert: false,
            }],
            mode: KnobMode::Relative,
            bidirectional: false,
            takeover_state: Arc::new(SoftTakeoverState::default()),
        };
        mapper.add_binding(binding);

        // Increment (Value 1)
        // Sensitivity is 0.02. Current norm 0.5 -> 0.52. Mapped 0.52 * 100 = 52.0
        let result = mapper.process_cc(1, 0, 10, 1, |_| 0.5);
        if let MappingResult::ParameterUpdates(updates) = result {
            assert_eq!(updates[0].1, 52.0);
        }

        // Decrement (Value 127 = -1)
        // Current norm 0.5 -> 0.48. Mapped 48.0
        let result = mapper.process_cc(1, 0, 10, 127, |_| 0.5);
        if let MappingResult::ParameterUpdates(updates) = result {
            assert_eq!(updates[0].1, 48.0);
        }
    }

    #[test]
    fn test_fourteen_bit_cc() {
        let mapper = NeuralMapper::new();
        let param_id = Uuid::new_v4();

        let binding = MidiBinding {
            id: Uuid::new_v4(),
            device_hash: 1,
            channel: 0,
            cc_number: 10, // Virtual ID or MSB
            resolution: MidiResolution::FourteenBit {
                msb_cc: 10,
                lsb_cc: 42,
            },
            targets: vec![ParameterTarget {
                param_id,
                min: 0.0,
                max: 1.0,
                scale: 1.0,
                invert: false,
            }],
            mode: KnobMode::Absolute,
            bidirectional: false,
            takeover_state: Arc::new(SoftTakeoverState::default()),
        };
        mapper.add_binding(binding);

        // 1. Send MSB only (Val 64 -> 0.5 approx)
        // With soft takeover, we need to be close to 0.0 (value_provider)
        // Let's set value_provider to 0.48 so 0.5 catches it.
        let result = mapper.process_cc(1, 0, 10, 64, |_| 0.48);
        // MSB 64, LSB 0 = 8192. 8192 / 16383 = 0.5
        if let MappingResult::ParameterUpdates(updates) = result {
            assert!((updates[0].1 - 0.5).abs() < 0.001);
        } else {
            panic!("Expected updates, got {:?}", result);
        }

        // 2. Send LSB (CC 42 for MSB 10)
        let result = mapper.process_cc(1, 0, 42, 127, |_| 0.5);
        // MSB 64, LSB 127 = 8192 + 127 = 8319. 8319 / 16383 = 0.5077
        if let MappingResult::ParameterUpdates(updates) = result {
            assert!((updates[0].1 - 0.5077).abs() < 0.001);
        }
    }

    #[test]
    fn test_nrpn_resolution() {
        let mapper = NeuralMapper::new();
        let param_id = Uuid::new_v4();

        // NRPN requires pairing: CC 99 (MSB), CC 98 (LSB), CC 6 (Data)
        let binding = MidiBinding {
            id: Uuid::new_v4(),
            device_hash: 1,
            channel: 0,
            cc_number: 127, // Dummy for NRPN
            resolution: MidiResolution::Nrpn { msb: 8, lsb: 0 },
            targets: vec![ParameterTarget {
                param_id,
                min: 0.0,
                max: 1.0,
                scale: 1.0,
                invert: false,
            }],
            mode: KnobMode::Absolute,
            bidirectional: false,
            takeover_state: Arc::new(SoftTakeoverState::default()),
        };
        mapper.add_binding(binding);

        // Simulate NRPN sequence
        let _ = mapper.process_cc(1, 0, 99, 8, |_| 0.0); // MSB
        let _ = mapper.process_cc(1, 0, 98, 0, |_| 0.0); // LSB
                                                         // Soft takeover: HW 0.5, current 0.48 -> Engage
        let result = mapper.process_cc(1, 0, 6, 64, |_| 0.48); // Data MSB (6144)

        if let MappingResult::ParameterUpdates(updates) = result {
            // Note: NRPN ID 1024 == MSB 8, LSB 0
            // Data 64 == 8192 / 16383 = 0.5
            assert!((updates[0].1 - 0.5).abs() < 0.01);
        } else {
            panic!("Expected updates, got {:?}", result);
        }
    }
}
