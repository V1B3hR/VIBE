import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, cleanup, act, fireEvent } from '@testing-library/react';
import React from 'react';

// ============================================================================
// VIBE MIGHTY BACKEND SIMULATOR
// ============================================================================
const { simulator } = vi.hoisted(() => {
    const createTrack = (id: string, name: string) => ({
        id,
        name,
        volume: { id: `v-${id}`, name: 'Volume', value: 0.8, min_value: -60, max_value: 6, automation: [] },
        is_muted: false,
        is_solo: false,
        is_disabled: false,
        is_frozen: false,
        clips: [],
        midi_clips: [],
        color: '#ff0000',
        peak_l: -144,
        peak_r: -144,
        rms_l: -144,
        rms_r: -144,
        effects: [],
        pan: { id: `p-${id}`, name: 'Pan', value: 0.0 },
        width: { id: `w-${id}`, name: 'Width', value: 1.0 },
        input_drive: { id: `d-${id}`, name: 'Drive', value: 0.0 },
        console_eq: { id: `eq-${id}`, name: 'EQ', parameters: [] },
        console_comp: { id: `cp-${id}`, name: 'Comp', parameters: [] }
    });

    return {
        simulator: {
            state: {
                isPlaying: false,
                isRecording: false,
                playhead: 0,
                bpm: 120,
                tracks: [createTrack('t0', 'Synth 1')],
                markers: [],
                loopRange: [0, 48000 * 4],
                listeners: {} as Record<string, ((event: any) => void)[]>
            },
            reset() {
                this.state.isPlaying = false;
                this.state.isRecording = false;
                this.state.playhead = 0;
                this.state.tracks = [createTrack('t0', 'Synth 1')];
                this.state.listeners = {};
            },
            async invoke(cmd: string, args: any) {
                switch (cmd) {
                    case 'get_tracks': return this.state.tracks;
                    case 'get_bpm': return this.state.bpm;
                    case 'get_playhead': return this.state.playhead;
                    case 'is_playing': return this.state.isPlaying;
                    case 'is_recording': return this.state.isRecording;
                    case 'get_markers': return this.state.markers;
                    case 'get_loop_range': return this.state.loopRange;
                    case 'is_loop_enabled': return false;
                    case 'get_cpu_load': return 12.5;
                    case 'get_memory_usage': return 256.0;
                    case 'get_track_levels': return [];
                    case 'get_master_meters': return { peak_l_db: -60, peak_r_db: -60, rms_l_db: -60, rms_r_db: -60 };
                    case 'get_midi_bindings': return [];
                    case 'play_audio': this.state.isPlaying = true; return null;
                    case 'pause_audio': this.state.isPlaying = false; return null;
                    case 'add_track':
                        const newId = `t${this.state.tracks.length}`;
                        this.state.tracks.push(createTrack(newId, args.name || `Track ${this.state.tracks.length + 1}`));
                        return null;
                    case 'set_track_mute':
                        if (this.state.tracks[args.index]) {
                            this.state.tracks[args.index].is_muted = args.muted;
                        }
                        return null;
                    default: return null;
                }
            },
            async listen(event: string, callback: (event: any) => void) {
                if (!this.state.listeners[event]) this.state.listeners[event] = [];
                this.state.listeners[event].push(callback);
                return () => {
                    this.state.listeners[event] = this.state.listeners[event]?.filter(l => l !== callback);
                };
            },
            emit(event: string, payload: any) {
                if (this.state.listeners[event]) {
                    this.state.listeners[event].forEach(l => l({ payload }));
                }
            }
        }
    };
});

vi.mock('@tauri-apps/api/core', () => ({
    invoke: vi.fn((cmd, args) => simulator.invoke(cmd, args)),
}));
vi.mock('@tauri-apps/api/event', () => ({
    listen: vi.fn((ev, cb) => simulator.listen(ev, cb)),
}));

// Exhaustive Mocks
vi.mock('../components/SampleEditor', () => ({ SampleEditor: () => null }));
vi.mock('../components/WaveformGL', () => ({ WaveformGL: () => null }));
vi.mock('../components/TimelineRuler', () => ({ TimelineRuler: () => null }));
vi.mock('../components/OverviewBar', () => ({ OverviewBar: () => null }));
vi.mock('../components/LivingFader', () => ({ LivingFader: () => null }));
vi.mock('../components/EqCanvas', () => ({ EqCanvas: () => null }));
vi.mock('../components/CompCanvas', () => ({ CompCanvas: () => null }));
vi.mock('../components/PluginRackUnit', () => ({ PluginRackUnit: () => null }));
vi.mock('../components/PianoRoll', () => ({ PianoRoll: () => null }));
vi.mock('../components/AutomationLane', () => ({ AutomationLane: () => null }));
vi.mock('../components/TimelineContextMenu', () => ({ TimelineContextMenu: () => null }));
vi.mock('../components/TrackContextMenu', () => ({ TrackContextMenu: () => null }));
vi.mock('../components/ClipContextMenu', () => ({ ClipContextMenu: () => null }));
vi.mock('../components/NanoEq', () => ({ NanoEq: () => null }));
vi.mock('../components/NanoComp', () => ({ NanoComp: () => null }));
vi.mock('../components/DriveKnob', () => ({ DriveKnob: () => null }));
vi.mock('../components/MicroScope', () => ({ MicroScope: () => null }));
vi.mock('../components/TubeLimiterCanvas', () => ({ TubeLimiterCanvas: () => null }));
vi.mock('../components/FilterCanvas', () => ({ FilterCanvas: () => null }));
vi.mock('../components/ReverbCanvas', () => ({ ReverbCanvas: () => null }));
vi.mock('../components/DelayCanvas', () => ({ DelayCanvas: () => null }));
vi.mock('../components/SaturationCanvas', () => ({ SaturationCanvas: () => null }));
vi.mock('../components/SynthCanvas', () => ({ SynthCanvas: () => null }));
vi.mock('../components/MagnetoSettings', () => ({ MagnetoSettings: () => null }));

// Mock ResizeObserver
global.ResizeObserver = class {
    observe() { }
    unobserve() { }
    disconnect() { }
};

import { Transport } from '../components/Transport';
import { Timeline } from '../components/Timeline';
import { Mixer } from '../components/Mixer';
import { MidiLearnProvider } from '../context/MidiLearnContext';

describe('VIBE High-Voltage E2E', () => {
    beforeEach(() => {
        simulator.reset();
        vi.useRealTimers();
    });

    afterEach(cleanup);

    it('manages a complex arrangement workflow: playback -> expand -> sync', async () => {
        render(
            <MidiLearnProvider>
                <div className="vibe-app-host">
                    <Transport />
                    <Timeline />
                    <Mixer />
                </div>
            </MidiLearnProvider>
        );

        // 1. Initial State
        const trackLabels = await screen.findAllByText(/Synth 1/i, {}, { timeout: 10000 });
        expect(trackLabels.length).toBeGreaterThan(0);
        console.log("⚡ Bolt: App loaded successfully.");

        // 2. Playback
        const playBtn = screen.getByTestId('transport-play');
        await act(async () => {
            fireEvent.click(playBtn);
        });

        // Wait for state update
        await new Promise(r => setTimeout(r, 100));
        expect(simulator.state.isPlaying).toBe(true);
        console.log("⚡ Bolt: Global playback engaged.");

        // 3. Arrangement Expansion
        const addBtn = screen.getByText(/\+ Audio Track/i);
        await act(async () => {
            fireEvent.click(addBtn);
        });

        // Backend Sync
        await act(async () => {
            simulator.emit('project_updated', {
                tracks: simulator.state.tracks,
                bpm: simulator.state.bpm,
                markers: simulator.state.markers
            });
        });

        const newLabels = await screen.findAllByText(/Track 2/i, {}, { timeout: 5000 });
        expect(newLabels.length).toBeGreaterThanOrEqual(1);

        // 4. Mute Action (Using Mixer buttons which are reliably rendered)
        const muteBtns = await screen.findAllByTitle("Mute", {}, { timeout: 3000 });

        // We want the mute button for the second track (index 1)
        if (muteBtns.length < 2) throw new Error("Expected at least 2 mute buttons in Mixer");
        const targetBtn = muteBtns[1];

        await act(async () => {
            fireEvent.click(targetBtn);
        });

        await act(async () => {
            simulator.emit('project_updated', { tracks: simulator.state.tracks });
        });

        expect(simulator.state.tracks[1].is_muted).toBe(true);
    }, 45000);
});
