# VIBE DAW - API Reference & Event Bus Protocol

**Version:** `v0.5.0-beta`  
**Target:** Tauri 2.0 Inter-Process Communication (IPC) Bridge  

---

## 1. Overview

The VIBE DAW backend (written in Rust) communicates with the frontend UI (React 18 + TypeScript) via Tauri 2.0 IPC `invoke` commands and asynchronous event broadcasts (`emit` / `listen`).

---

## 2. Core Tauri IPC Commands

### 2.1 Transport & Clock Commands
| Command | Arguments | Return Type | Description |
| :--- | :--- | :--- | :--- |
| `play_audio` | None | `void` | Starts audio playback from current playhead position |
| `pause_audio` | None | `void` | Pauses audio playback |
| `stop_audio` | None | `void` | Stops playback and resets playhead to zero |
| `is_playing` | None | `boolean` | Queries if transport is currently playing |
| `get_playhead` | None | `number` | Returns playhead position in audio samples |
| `seek_playhead` | `pos: number` | `void` | Seeks playhead to specific sample position |
| `set_bpm` | `bpm: number` | `void` | Sets project tempo (20.0 to 999.0 BPM) |
| `get_bpm` | None | `number` | Returns current project tempo |
| `set_loop_range` | `start: number, end: number` | `void` | Configures loop start and end sample markers |
| `get_loop_range` | None | `[number, number]` | Returns `[start_sample, end_sample]` array |

---

### 2.2 Track & Routing Commands
| Command | Arguments | Return Type | Description |
| :--- | :--- | :--- | :--- |
| `get_tracks` | None | `Track[]` | Fetches complete project track hierarchy |
| `add_track` | `name: string` | `void` | Appends new audio track to the project |
| `create_track` | `name: string, track_type: string` | `string` | Creates specific track (Audio, MIDI, Bus, Folder) |
| `remove_track` | `index: number` | `void` | Removes track at specified index |
| `duplicate_track` | `index: number` | `void` | Duplicates track with all clips and settings |
| `rename_track` | `index: number, name: string` | `void` | Renames specified track |
| `set_track_volume` | `index: number, volume: number` | `void` | Sets linear track volume (0.0 to 1.0+) |
| `set_track_pan` | `index: number, pan: number` | `void` | Sets track panning (-1.0 Left to 1.0 Right) |
| `set_track_width` | `index: number, width: number` | `void` | Sets stereo width (0.0 Mono to 2.0 Extra) |
| `set_track_mute` | `index: number, muted: boolean` | `void` | Sets track mute state |
| `set_track_solo` | `index: number, solo: boolean` | `void` | Sets track solo state |
| `set_track_arm` | `index: number, armed: boolean` | `void` | Arms track for recording |

---

### 2.3 Clips & Arrangement Commands
| Command | Arguments | Return Type | Description |
| :--- | :--- | :--- | :--- |
| `add_clip_to_track` | `trackIdx: number, filePath: string, startSample: number` | `string` | Imports and adds audio clip to track |
| `add_midi_clip` | `trackIdx: number, clip: MidiClip` | `string` | Adds new MIDI clip to specified track |
| `move_clip` | `trackIndex: number, clipId: string, newStartSample: number` | `void` | Repositions clip on timeline |
| `resize_clip` | `trackIndex: number, clipId: string, newDuration: number` | `void` | Resizes clip duration |
| `slice_clip` | `trackIndex: number, clipId: string, samplePos: number` | `void` | Splits clip into two at specified sample position |
| `delete_clip` | `trackIndex: number, clipId: string` | `void` | Removes clip from track |
| `set_clip_gain` | `trackIdx: number, clipId: string, gainDb: number` | `void` | Sets non-destructive clip gain in dB |

---

### 2.4 Metering & Telemetry
| Command | Arguments | Return Type | Description |
| :--- | :--- | :--- | :--- |
| `get_track_levels` | None | `TrackLevel[]` | Fast polling for RMS, Peak, and LUFS per track |
| `get_master_meters` | None | `MasterMeters` | Returns Master L/R Peak, RMS, True Peak & LUFS |
| `get_spectrum_data` | `trackIndex: number` | `number[]` | 128-band FFT spectral data frame |

---

### 2.5 Kropelka AI & Assistant Commands
| Command | Arguments | Return Type | Description |
| :--- | :--- | :--- | :--- |
| `ask_kropelka` | `context: KropelkaContext` | `KropelkaBrainResponse` | Queries neural assistant for mixing/theory advice |
| `apply_kropelka_fix` | `action_type: string, action_data: any` | `string` | Executes automated fix recommended by Kropelka |
| `get_kropelka_stats` | None | `KropelkaStats` | Retrieves emotional state and affinity statistics |
| `reset_kropelka_memory` | None | `void` | Wipes assistant memory state |

---

## 3. Asynchronous Event Bus

Tauri broadcasts events from the backend to the UI layer over the IPC event channel:

```
[Rust Engine] ──(emit)──> [Tauri Event Bus] ──(listen)──> [React Subscriptions]
```

### Registered Events

1. **`project_updated`**
   - **Trigger:** Fired whenever track topology, clip placement, or plugin chains change.
   - **Payload:** `null` (signals UI to trigger state refresh via `get_tracks`).

2. **`audio_device_changed`**
   - **Trigger:** Audio device plug/unplug, sample rate change, or buffer size modification.
   - **Payload:** `AudioDeviceConfig` object.

3. **`kropelka_insight`**
   - **Trigger:** Proactive background detection of clipping, phase cancellation, or mud in mix.
   - **Payload:** `InsightCard` object with recommended action.

4. **`midi_message_received`**
   - **Trigger:** Incoming hardware MIDI message on armed track.
   - **Payload:** `[status, data1, data2]` array.

---

## 4. Error Handling Protocol

All commands return standard Rust `Result<T, String>` serialized into promises:
- On success: Resolves with return value `T`.
- On failure: Rejects with error message string `err` describing the cause (e.g. `"Device Busy"`, `"Clip Not Found"`).
