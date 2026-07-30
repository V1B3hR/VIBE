# Test Plan: VIBE DAW V2

## Overview
This document outlines the comprehensive testing strategy for VIBE DAW V2. It modernizes the previous implementation plan to include recently added features such as the Spectral Engine, Audio-to-MIDI conversion, and the new UI components.

## Target Coverage
*   **Backend (Rust)**: 90%+ Line Coverage
    *   Focus: Audio Engine, Spectral Analysis, MIDI generation, Plugin Hosting.
*   **Frontend (React/TS)**: 80%+ Component Coverage
    *   Focus: Timeline logic, Sample Editor, State Management.
*   **E2E**: Critical User Journeys (Import, Edit, Export).

---

## 1. Backend Testing Strategy (Rust) - [MODERNIZED]

### 1.1 New Modules (Priority: High)
*   **`processor.rs` (MelProcessor)**: ✅ Done. Sine/Silence tests passing.
*   **`transcription.rs` (Transcriber)**: ✅ Done. Peak detection tests passing.
*   **`worker.rs` (SpectralWorker)**: ✅ Done. Integration/concurrency test passing.
*   **`drum_detector.rs`**: ✅ Done. Kick/Hat detection passing.
*   **`onset_detector.rs`**: ✅ Done. Spectral flux analysis passing.

### 1.2 Audio-to-MIDI (`src-tauri/src/engine/audio_to_midi.rs`)
*   **Polyphonic Conversion**: ✅ Done. Simple conversion test passing.

### 1.3 Time & Pitch
*   **Pitch Detection**: ✅ Done. Refined YIN algorithm with parabolic interpolation.

### 1.4 Core Graph & Optimization
*   **`graph.rs` / `graph_tests.rs`**: ✅ Done. Basic buffer/gain/filter tests passing.
*   **`simd.rs`**: ✅ Done. Vectorized math correctness verified (Mix, Gain, Saturation).

### 1.5 Plugin Browser & Streaming
*   **`plugin_manager.rs`**: ✅ Done. Categorization, Tags, Favorites, Blacklist fully tested.
*   **`wasm.rs`**: ✅ Done. Plugin hosting, parameter sync, memory processing verified.
*   **`velocity.rs`**: ✅ Done. SmartClip metadata and StreamingVoice logic verified.

---

## 2. Frontend Testing Strategy (React/TypeScript) - [MODERNIZED]

### 2.1 UI Component Coverage (Interactive Controls)
*   **`Mixer.tsx`**: ✅ Done. All solo/arm/phase/mute buttons and FX inserts verified.
*   **`Transport.tsx`**: ✅ Done. Play/Stop/Rec/BPM/Metronome/Undo/Redo fully verified.
*   **`Library.tsx`**: ✅ Done. Tab switching, Imports, Track/Group creation verified.
*   **`PluginBrowser.tsx`**: ✅ Done. Categories, Tags, Folders, Favorites, Search, and Rescan verified.
*   **`SampleEditor.tsx`**: ✅ Done. View switching, spectral analysis trigger, and processing UI verified.
*   **Interactive Knobs/Faders**: ✅ Done. `ParamKnob`, `DriveKnob`, `LivingFader`, and `MicroScope` (Vector Pan) fully verified with drag/reset tests.
*   **`MelSpectrogram.tsx`**: ✅ Done. Units/Rendering tests passing.
*   **`Timeline.tsx`**: ✅ Resolved. Brittle tests fixed with robust mocking.
*   **`MasterMeters.tsx`**: ✅ Done. Visual rendering and live meter update tests passing.

### 2.2 Error Handling
*   **`ErrorRecovery.test.tsx`**: ✅ Done. Backend crash handling verified.

---

## 3. Integration & E2E Tests
*   **Workflow Tests**: ✅ Done. E2E foundation implemented (Playback, Expand, Sync).
*   **Performance Benchmarks**: ✅ Done. "Traffic Jam" load tests for spectral data (8 tracks @ 60Hz) verified.
