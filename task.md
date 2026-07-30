# V-One Synth Upgrade: Modulation Matrix & Presets

## 1. Modulation Matrix Backend (Rust)
- [x] Define `ModSource` and `ModDest` Enums.
- [x] Add `mod_matrix` to `VOneSynth` Struct.
- [x] Implement `process` loop modulation application.
- [x] Remove hardwired Macro/Sequencer logic.
- [x] Fix compilation errors and lints.

## 2. Preset System Backend (Rust)
- [x] Define `SynthPreset` Struct (Serializable/Deserializable).
- [x] Implement `get_current_patch() -> SynthPreset` (Gather all Params + Matrix).
- [x] Implement `set_patch(SynthPreset)` (Apply Params + Matrix).
- [x] Implement `save_to_json` and `load_from_json` utilities.
- [ ] Create Factory Presets (Rust-side defaults or JSON files).

## 3. Frontend (React)
- [ ] `ModMatrix.tsx`: Grid UI for connection management.
- [ ] `PresetBrowser.tsx`: UI for listing, saving, and loading presets.
- [ ] Updated `SynthCanvas.tsx` to include new tabs/panels.

## 4. Testing & Verification
- [ ] Verify Patch Saving/Loading Persistence.
- [ ] Verify Mod Matrix audio effect.
- [ ] Verify CPU usage with full matrix.
