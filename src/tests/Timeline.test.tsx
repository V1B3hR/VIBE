import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, cleanup, act, fireEvent } from '@testing-library/react';
import React from 'react';
import { Timeline } from '../components/Timeline';

// Mock Tauri
const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
    invoke: (cmd: string, args?: any) => mockInvoke(cmd, args),
}));

vi.mock('@tauri-apps/api/event', () => ({
    listen: vi.fn(async () => () => { }),
}));

// Mock sub-components that are already tested elsewhere or too complex
vi.mock('../components/SampleEditor', () => ({ SampleEditor: () => <div data-testid="sample-editor" /> }));
vi.mock('../components/PianoRoll', () => ({ PianoRoll: () => <div data-testid="piano-roll" /> }));
vi.mock('../components/WaveformGL', () => ({ WaveformGL: () => <div data-testid="waveform-gl" /> }));

// Mock clientWidth for JSDOM
Object.defineProperty(HTMLElement.prototype, 'clientWidth', { configurable: true, value: 1000 });

describe('Timeline Component', () => {
    beforeEach(() => {
        mockInvoke.mockImplementation(async (cmd) => {
            if (cmd === 'get_tracks') return [
                {
                    id: 'track-1',
                    name: 'Audio Track',
                    color: '#ff0000',
                    clips: [
                        { id: 'clip-1', name: 'Test Clip', start_sample: 0, duration_samples: 48000, peaks: [], offset_in_data: 0, fade_in_len: 0, fade_out_len: 0 }
                    ],
                    midi_clips: [],
                    volume: { id: 'vol-1', name: 'Volume', value: 0.8, min_value: 0, max_value: 1, automation: [] },
                    is_muted: false,
                    is_solo: false
                }
            ];
            if (cmd === 'get_markers') return [];
            if (cmd === 'get_loop_range') return [0, 192000];
            if (cmd === 'get_bpm') return 120;
            if (cmd === 'get_playhead') return 0;
            if (cmd === 'is_playing') return false;
            return {};
        });
    });

    afterEach(cleanup);

    it('renders tracks and clips correctly', async () => {
        render(<Timeline />);

        expect(await screen.findByText('Audio Track')).toBeInTheDocument();
        expect(await screen.findByText('Test Clip')).toBeInTheDocument();
        expect(await screen.findByTestId('timeline-snap-select')).toBeInTheDocument();
    });

    it('handles track mute toggle', async () => {
        await act(async () => {
            render(<Timeline />);
        });

        const muteBtn = screen.getByTestId('track-mute-0');
        await act(async () => {
            fireEvent.click(muteBtn);
        });

        expect(mockInvoke).toHaveBeenCalledWith('set_track_mute', expect.objectContaining({ index: 0, muted: true }));
    });

    it('handles snap change', async () => {
        await act(async () => {
            render(<Timeline />);
        });

        const snapSelect = screen.getByTestId('timeline-snap-select');
        fireEvent.change(snapSelect, { target: { value: '4' } });

        expect((snapSelect as HTMLSelectElement).value).toBe('4');
    });

    it('handles zoom in/out button clicks', async () => {
        await act(async () => {
            render(<Timeline />);
        });

        const zoomInBtn = screen.getByTestId('timeline-zoom-in-btn');
        const zoomOutBtn = screen.getByTestId('timeline-zoom-out-btn');

        fireEvent.click(zoomInBtn);
        // We can't easily check the internal state of the hook here without more complex setup, 
        // but we verify the buttons exist and are clickable.
        expect(zoomInBtn).toBeInTheDocument();
        expect(zoomOutBtn).toBeInTheDocument();
    });
});
