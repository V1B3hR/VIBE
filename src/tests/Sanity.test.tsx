import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, cleanup, act } from '@testing-library/react';
import React from 'react';
import { Transport } from '../components/Transport';
import { Timeline } from '../components/Timeline';
import { Mixer } from '../components/Mixer';
import { MidiLearnProvider } from '../context/MidiLearnContext';

// Mock Tauri
vi.mock('@tauri-apps/api/core', () => ({
    invoke: vi.fn(async (cmd) => {
        if (cmd === 'get_tracks') return [];
        if (cmd === 'get_markers') return [];
        if (cmd === 'get_loop_range') return [0, 1000];
        if (cmd === 'get_bpm') return 120;
        if (cmd === 'get_track_levels') return [];
        if (cmd === 'get_master_meters') return { peak_l_db: -60, peak_r_db: -60, rms_l_db: -60, rms_r_db: -60 };
        if (cmd === 'get_midi_bindings') return [];
        return {};
    }),
}));
vi.mock('@tauri-apps/api/event', () => ({
    listen: vi.fn(async () => () => { }),
}));

// Mock sub-components
vi.mock('../components/SampleEditor', () => ({ SampleEditor: () => null }));
vi.mock('../components/WaveformGL', () => ({ WaveformGL: () => null }));
vi.mock('../components/TimelineRuler', () => ({ TimelineRuler: () => null }));
vi.mock('../components/OverviewBar', () => ({ OverviewBar: () => null }));
vi.mock('../components/LivingFader', () => ({ LivingFader: () => null }));
vi.mock('../components/EqCanvas', () => ({ EqCanvas: () => null }));
vi.mock('../components/CompCanvas', () => ({ CompCanvas: () => null }));
vi.mock('../components/TubeLimiterCanvas', () => ({ TubeLimiterCanvas: () => null }));
vi.mock('../components/FilterCanvas', () => ({ FilterCanvas: () => null }));
vi.mock('../components/ReverbCanvas', () => ({ ReverbCanvas: () => null }));
vi.mock('../components/DelayCanvas', () => ({ DelayCanvas: () => null }));
vi.mock('../components/SaturationCanvas', () => ({ SaturationCanvas: () => null }));
vi.mock('../components/SynthCanvas', () => ({ SynthCanvas: () => null }));
vi.mock('../components/MagnetoSettings', () => ({ MagnetoSettings: () => null }));
vi.mock('../components/NanoEq', () => ({ NanoEq: () => null }));
vi.mock('../components/NanoComp', () => ({ NanoComp: () => null }));
vi.mock('../components/DriveKnob', () => ({ DriveKnob: () => null }));
vi.mock('../components/MicroScope', () => ({ MicroScope: () => null }));
vi.mock('../components/PluginRackUnit', () => ({ PluginRackUnit: () => null }));

describe('Sanity Check', () => {
    afterEach(cleanup);

    it('renders everything together with Provider', async () => {
        await act(async () => {
            render(
                <MidiLearnProvider>
                    <div>
                        <Transport />
                        <Timeline />
                        <Mixer />
                    </div>
                </MidiLearnProvider>
            );
        });

        expect(screen.getByTestId('transport-play')).toBeInTheDocument();
        expect(screen.getByText(/SNAP:/i)).toBeInTheDocument();
        expect(screen.getByText(/VIBE CONSOLE/i)).toBeInTheDocument();
    });
});
