# VIBE DAW - Implementation Plan

## Phase 1: Core Engine & Foundation (Current Phase)

### 1.1 Audio Engine Core (Rust)
- [x] **Audio Thread Architecture**: Dedicated high-priority thread using `cpal`.
- [x] **Graph Processor**: Track -> Bus -> Master routing system.
- [x] **Internal Buffering**: Zero-allocation audio buffer handling (`AudioBuffer`).
- [x] **Parameter System**: Automatable parameters (`Parameter` struct with interpolators).
- [x] **Simd Optimizations**: `simd.rs` module for vector operations (AVX/SSE).
- [x] **"Sport Mode" Telemetry**:
  - [x] Real-time CPU Load Metering (Atomic timers).
  - [x] Latency monitoring.

### 1.2 Infinite History System (Project DAG)
- [x] **Core Logic**: `HistoryManager` with Git-like DAG structure (Parents, Children).
- [x] **Snapshotting**: Efficient state capture (`ProjectSnapshot`) on action.
- [x] **Branching**: Support for alternative timelines/branches.
- [x] **Engine Integration**: `undo()`, `redo()`, `checkout()` commands directly in Audio Engine.
- [x] **UI Visualization**: `HistoryGraph` component showing the commit tree.

### 1.3 WASM & Native Plugin Architecture
- [x] **Sandboxing**: `wasmer` runtime integration for safe plugin execution.
- [x] **Memory Layout**: Shared linear memory for zero-copy audio passing.
- [x] **Processor Wrapper**: `WasmAudioProcessor` implementing the `AudioProcessor` trait.
- [x] **Plugin Discovery**: Scanning and loading `.wasm` files from directory.
- [x] **Plugin Loading**: Drag & Drop from Library to Mixer.
- [x] **Native VST Bridge**: Support for `.dll` plugins (VST2). `.vst3` skeleton ready.
- [x] **Parameter Reflection**: System to query plugin parameters (Min/Max/Value/Name).

### 1.4 MIDI 2.0 & Control
- [x] **UMP Support**: Internal `Ump` struct for Universal MIDI Packets.
- [x] **Legacy Bridge**: `Midi1ToUmp` converter for standard controllers.
- [x] **Mapping System**: `learn_midi` and parameter mapping logic.
- [x] **Transport Control**: Start/Stop/Record via MIDI.

### 1.5 User Interface (React)
- [x] **Transport Bar**: Play, Stop, Record, BPM, **CPU Meter**, **Undo/Redo**.
- [x] **Mixer View**: Volume faders, routing (Basic), **Mute/Solo**, **Plugin DnD**.
- [x] **Timeline View**: Track lanes, playhead visualization.
- [x] **History Panel**: Interactive graph of project states.
- [x] **Track Headers**: Mute/Solo buttons.
- [x] **Automation Lanes**: UI for drawing knots and curves.
- [x] **Plugin Rack UI**: Generic UI for WASM and VST parameters in Mixer.
- [x] **Real-time Recording**: Parameter recording into `AutomationCurve` active.
- [x] **Crash Recovery**: Auto-save system (`recovery.rs`) active.

---

## Phase 2: Arrangement & Workflow (Upcoming)

### 2.1 Timeline Interaction
- [x] **Smart Tool**: Context-sensitive cursor (Select, Slice).
- [x] **Drag & Drop**: Moving clips between tracks.
- [x] **Crossfades**: Automatic audio crossfading on clip overlap (Liquid Core).

### 2.2 Audio Clips
- [x] **Disk Streaming**: Reading large files from disk (Velocity Engine).
- [ ] **Waveform Rendering**: High-performance canvas rendering of peaks.
- [ ] **Time-Stretching**: Basic pitch-shift/stretch algorithms.

### 2.3 Automation
- [ ] **Automation Lanes**: UI for drawing bezier curves.
- [ ] **Recording**: Real-time parameter recording into `AutomationCurve`.

---

## Phase 3: Polish & Ecosystem

- [ ] **Theme Engine**: User-customizable CSS variables.
- [ ] **Keyboard Shortcuts**: Configurable keybindings.
- [ ] **Export/Bounce**: Rendering project to WAV/MP3.
