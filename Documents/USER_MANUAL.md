# 🎵 VIBE DAW - User Manual & Operating Guide

**Version:** `v0.5.0-beta`  
**Interface:** Spectral Glass & Neon Aesthetic  

---

## 1. Introduction

Welcome to **VIBE DAW**, a professional Digital Audio Workstation designed for high-performance music production, mixing, and sound design. VIBE combines a lock-free, zero-allocation Rust audio engine with a responsive React frontend and the **Kropelka AI** copilot.

---

## 2. Audio & Hardware Setup

### 2.1 Configuring Audio Driver
1. Click the **⚙️ Settings** icon in the transport bar or press <kbd>Ctrl</kbd> + <kbd>,</kbd>.
2. Select your Audio Driver Host:
   - **WASAPI** (Default low-latency driver on Windows)
   - **ASIO** (Recommended for professional audio interfaces like Focusrite, Universal Audio, Steinberg)
3. Select your output device and desired **Sample Rate** (48.0 kHz recommended) and **Buffer Size** (128 - 512 samples).
4. Lower buffer sizes decrease round-trip latency; higher buffer sizes provide stability during heavy sessions.

---

## 3. Arrangement View & Timeline

### 3.1 Navigating the Timeline
- **Horizontal Zoom**: <kbd>Ctrl</kbd> + <kbd>Mouse Scroll</kbd> or <kbd>+</kbd> / <kbd>-</kbd> buttons on the toolbar.
- **Vertical Zoom**: Adjust track heights by dragging the bottom border of any track header.
- **Scrubbing**: Click or drag along the upper **Timeline Ruler** to position the playhead.

### 3.2 Editing Audio & MIDI Clips
- **Splitting Clips**: Position playhead at split point and press <kbd>S</kbd> or click ✂️ **Split**.
- **Duplicating**: Select clip and press <kbd>Ctrl</kbd> + <kbd>D</kbd>.
- **Clip Gain**: Drag the upper Gain handle on any audio clip to adjust clip volume in decibels with instant visual feedback.
- **Fades**: Drag top-left or top-right fade handles for crackle-free linear or exponential fades.
- **Looping**: Right-click ruler to set high-precision loop boundaries down to 1/128 bar grid intervals.

---

## 4. The Mixer Panel

Access the Mixer by pressing <kbd>Ctrl</kbd> + <kbd>M</kbd>.

### 4.1 Channel Strips & Living Faders
- **Living Faders**: Ultra-smooth volume faders with digital dB readouts.
- **Pan & Width**: Adjust stereo positioning (-1.0 Left to +1.0 Right) and width (0.0 Mono to 2.0 Wide).
- **Mute / Solo / Arm**: Toggle track states using dedicated M, S, R buttons.
- **Insert FX Rack**: Drag-and-drop built-in plugins or external VST3/WASM effects into insert slots.

### 4.2 Metering & Mastering Bus
- **Peak & RMS Meters**: Real-time signal level monitoring with peak hold indicators.
- **True Peak & LUFS**: Integrated, Short-Term, and Momentary loudness meters compliant with EBU R128 mastering standards.

---

## 5. Piano Roll & MIDI Sequencing

Press <kbd>Ctrl</kbd> + <kbd>E</kbd> or double-click a MIDI clip to open the Piano Roll.

### 5.1 Note Editing & Drawing
- **Pencil Tool**: Click to draw notes; drag right to set length.
- **Select / Move**: Click and drag notes; use arrow keys for semitone transpositions.
- **Velocity**: Adjust velocity bars in the lower lane (0 - 127).
- **MPE Support**: Edit pitch bend, pressure, and timbre curves per individual note.

---

## 6. V-One Synthesizer & Factory Presets

V-One is VIBE's flagship polyphonic virtual analog synthesizer.

- **Oscillators**: 3 Oscillators with Poly-BLEP anti-aliased waveforms (Saw, Pulse, Tri, Sine, Noise) and Super-Saw unison mode.
- **Modulation Matrix**: Assign LFOs, Envelopes, Sequencers, or MIDI Controllers to synth parameters (Filter Cutoff, Resonance, Pitch, FM Amount).
- **Anti-Aliasing Filter**: Smoothed 1-pole IIR filter tracking prevents digital zipper noise even under ultra-fast LFO rates.
- **Presets**: Load factory sounds (Leads, Basses, Pads, FX) via the preset browser `.vone` dropdown.

---

## 7. Kropelka AI Copilot 🧠

Kropelka is an integrated neural assistant that monitors session dynamics and offers real-time guidance.

- **Modes**:
  - **Creative**: Generates drum clips, chord suggestions, and arrangement ideas.
  - **Technical**: Detects clipping, phase issues, and frequency masking between tracks.
  - **Vibe Check**: Analyzes spectral balance and mix cohesion.
- **Interacting**: Click Kropelka's avatar in the bottom-right corner or press <kbd>Ctrl</kbd> + <kbd>K</kbd> to open the assistant chat window.
