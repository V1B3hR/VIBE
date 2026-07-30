# VIBE Timeline & Arrangement QA Protocol

This document outlines the comprehensive test strategy for the VIBE DAW Arrangement module. It covers the core DSP logic, the Tauri IPC bridge, and the React/Canvas frontend.

## 🟢 1. BACKEND (Rust / DSP Core) – The "Atomic Truth"
Testing logic, concurrency, and sample-accurate timing.

### A. Transport & Scheduling (Serce Czasu)
*   **Atomic Sample Counting**: [AUTOMATED 🟢]
    *   *Test*: Run the engine for 1 million cycles.
    *   *Verify*: (arrangement_tests.rs) Compare `playhead: AtomicU64` with total frames. No drift allowed.
*   **Loop Boundary Crossing**:
    *   *Test*: Set loop range [48000, 96000]. Playhead at 95990. Buffer size 1024.
    *   *Verify*: Does the engine correctly wrap the remaining 1014 samples to start at 48000? Check waveform continuity.
*   **Tempo Ramp Stress**:
    *   *Test*: Linear BPM automation from 60 to 240 BPM over 1 bar.
    *   *Verify*: `AudioEngine::bpm_atomic` updates and the grid resolution adapts without jitter.

### B. Clip Surgery (Operacje na danych)
*   **Non-Destructive Slicing (HyperStream Efficiency)**: [AUTOMATED 🟢]
    *   *Test*: Clone an `AudioClip` 1000 times (simulating `slice_clip`).
    *   *Verify*: (arrangement_tests.rs) Inspect memory usage. Arcs must point to same ptr.
*   **Crossfade Math (Equal Power Curves)**:
    *   *Test*: Crossfade between two white noise clips.
    *   *Verify*: Measure peak levels during the transition. The "dip" should follow the selected FadeType curve (Linear vs Logarithmic).
*   **Comping State Machine (Takes & Playlists)**: [AUTOMATED 🟢]
    *   *Test*: Rapidly switch `active_playlist_idx` during playback.
    *   *Verify*: (arrangement_tests.rs) Playlist index and metadata swapping logic.

### C. Track Management & Routing
*   **Clip Envelope Integrity**: [AUTOMATED 🟢]
    *   *Test*: Add knots to clip gain/pitch envelopes.
    *   *Verify*: (arrangement_tests.rs) Automation knot count and value precision.
*   **Freeze/Flatten Integrity**:
    *   *Test*: Record V-One Synth output to a new track. Phase-invert the original.
    *   *Verify*: Summing should result in silence (Null Test).
*   **Folder Grouping Latency**:
    *   *Test*: Route Audio -> Folder 1 -> Folder 2 -> Master.
    *   *Verify*: Check `PdcManager` (Plugin Delay Compensation) to ensure zero phase-shift between direct and grouped tracks.

---

## 🔵 2. BRIDGE (Tauri / IPC) – The "Nervous System"
Ensures the state remains synchronized between Rust and React.

*   **Heavy Load Serialization**:
    *   *Test*: Project with 50 tracks, 500 clips, and 2000 automation points.
    *   *Verify*: `emit_project_update` performance. Ensure JSON serialization does not block the UI thread for > 16ms (1 frame).
*   **Real-Time Automation Stream**:
    *   *Test*: Move a fader in the UI rapidly.
    *   *Verify*: Backend `AudioCommand` receiver queue depth. Ensure no packets are dropped and automation is smooth (no "zipper noise").
*   **Command Throttling**:
    *   *Test*: Drag a 2GB clip across the timeline.
    *   *Verify*: UI should update at 60Hz. Backend should receive throttled `MoveClip` updates for preview, and one final "commited" position.

---

## 🟠 3. FRONTEND (React / Canvas) – The "Pilot's Cockpit"
UX, precision, and rendering performance.

### A. Canvas Rendering & Performance
*   **The "Zoom Out" Stress Test**:
    *   *Test*: 3-hour project viewed at 100%.
    *   *Verify*: Monitor FPS. Ensure `Waverenderer.ts` uses Level of Detail (LOD) - drawing max/min envelopes instead of every single sample.
*   **Adaptive Grid Rendering**:
    *   *Test*: Dynamic Zoom from 1/4 to 1/128.
    *   *Verify*: Grid lines should fade in/out using CSS opacity or Canvas Alpha. No "flicker".
*   **Scrolling Smoothness**:
    *   *Test*: Vertical scroll through 100 tracks.
    *   *Verify*: Use `requestAnimationFrame`. No tearing or lag in waveform updates.

### B. Interaction & Tools
*   **Snap-to-Grid Magnetism**:
    *   *Test*: Set Global Quantization to `Triplet (1/16T)`.
    *   *Verify*: Mouse events should calculate `x = Math.round(rawX / tripletSize) * tripletSize`.
*   **Automation Curve Bezier**:
    *   *Test*: Use the "Tension" handle on an automation point.
    *   *Verify*: Visual curve must match the values sent in `SetClipEnvelope`.
*   **Ghost Notes Visibility**:
    *   *Test*: Edit MIDI Clip A.
    *   *Verify*: MIDI Clip B on the adjacent track should be visible as semitransparent grey notes.

---

## 🔴 4. INTEGRATION SCENARIOS (The "Vibe Check")
Realistic stress scenarios.

*   **"The Panic Split"**: [AUTOMATED 🟢]
    *   *Scenario*: Start playback. Mash the `S` key (Split) repeatedly at different playhead positions.
    *   *Expectation*: (arrangement_tests.rs) Zero audio glitches. `HyperStreamer` handles new buffer offsets instantly.
*   **"The Undo Loop"**:
    *   *Scenario*: Delete a complex FX Chain. Undo (Ctrl+Z).
    *   *Expectation*: Full state restoration. All VST/Wasm parameters and automation knots must be identical to pre-deletion.
*   **"The Stretch Artifact"**:
    *   *Scenario*: Change Project Tempo from 120 to 140 BPM with clips in `Warp: Beats` mode.
    *   *Expectation*: Transients (drum hits) must stay perfectly on the grid. Only the timing between transients changes.
