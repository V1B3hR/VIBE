import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup, act } from '@testing-library/react';
import { Timeline } from './Timeline';

// Mock Tauri
const mockInvoke = vi.fn();
const mockListen = vi.fn().mockResolvedValue(() => { });

// Comprehensive bridge mock
vi.mock('@tauri-apps/api/core', () => ({
    invoke: vi.fn((cmd: string, args: any) => mockInvoke(cmd, args)),
}));

vi.mock('@tauri-apps/api/event', () => ({
    listen: vi.fn((event: string, handler: any) => mockListen(event, handler)),
}));

// Prevent Tauri internals error
(global as any).window.__TAURI_INTERNALS__ = {
    transformCallback: vi.fn(),
    invoke: vi.fn(),
    metadata: {}
};

// Mock child components
vi.mock('./SampleEditor', () => ({ SampleEditor: () => <div data-testid="mock-sample-editor" /> }));
vi.mock('./PianoRoll', () => ({ PianoRoll: () => <div data-testid="mock-piano-roll" /> }));
vi.mock('./AutomationLane', () => ({ AutomationLane: () => <div data-testid="mock-automation-lane" /> }));
vi.mock('./WaveformGL', () => ({ WaveformGL: () => <div data-testid="mock-waveform-gl" /> }));
vi.mock('./ClipContextMenu', () => ({ ClipContextMenu: () => <div data-testid="mock-context-menu" /> }));
vi.mock('./TimelineRuler', () => ({ TimelineRuler: () => <div data-testid="mock-ruler" /> }));
vi.mock('./OverviewBar', () => ({ OverviewBar: () => <div data-testid="mock-overview" /> }));
vi.mock('./TimelineContextMenu', () => ({ TimelineContextMenu: () => <div data-testid="mock-timeline-context-menu" /> }));
vi.mock('./TrackContextMenu', () => ({ TrackContextMenu: () => <div data-testid="mock-track-context-menu" /> }));

describe('Timeline Component', () => {
    const mockTracks = [
        {
            id: 'track-1',
            name: 'Audio Track 1',
            volume: { id: 'v1', name: 'Volume', value: 0.8, min_value: 0, max_value: 1, automation: [] },
            is_muted: false,
            is_solo: false,
            clips: [
                { id: 'clip-1', name: 'Sample 1', start_sample: 0, duration_samples: 48000, peaks: [[]], offset_in_data: 0, fade_in_len: 0, fade_out_len: 0, fade_in_type: 'linear', fade_out_type: 'linear' }
            ],
            midi_clips: [],
            color: '#ff0000'
        }
    ];

    beforeEach(() => {
        vi.clearAllMocks();

        mockInvoke.mockImplementation(async (cmd, args) => {
            if (cmd === 'get_tracks') return mockTracks;
            if (cmd === 'get_bpm') return 120;
            if (cmd === 'get_playhead') return 0;
            if (cmd === 'is_playing') return false;
            if (cmd === 'get_markers') return [];
            if (cmd === 'get_loop_range') return [0, 48000 * 4];
            return null;
        });

        // Mock crypto.randomUUID
        Object.defineProperty(global, 'crypto', {
            value: {
                randomUUID: () => 'test-uuid'
            },
            configurable: true
        });

        global.Image = class {
            constructor() { }
            set src(s: string) { }
        } as any;
        global.innerWidth = 1024;
        global.innerHeight = 768;
    });

    afterEach(cleanup);

    it('renders and fetches tracks', async () => {
        render(<Timeline />);

        await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith('get_tracks', undefined));
        expect(await screen.findByText('Audio Track 1')).toBeInTheDocument();
        expect(screen.getByTestId('clip-clip-1')).toBeInTheDocument();
    });

    it('toggles track mute', async () => {
        render(<Timeline />);
        const muteBtn = await screen.findByTestId('track-mute-0');

        await act(async () => {
            fireEvent.click(muteBtn);
        });

        expect(mockInvoke).toHaveBeenCalledWith('set_track_mute', { index: 0, muted: true });
    });

    it('handles snap selection', async () => {
        render(<Timeline />);
        const snapSelect = await screen.findByTestId('timeline-snap-select');

        await act(async () => {
            fireEvent.change(snapSelect, { target: { value: '4' } });
        });

        expect(snapSelect).toHaveValue('4');
    });

    it('calls split on button click', async () => {
        render(<Timeline />);

        // Wait for tracks to be loaded — use findByTestId to let the ref-sync effect fire
        await screen.findByText('Audio Track 1');

        // Extra tick so tracksRef.current is populated from the useEffect sync
        const splitBtn = await screen.findByTestId('timeline-split-btn');

        await act(async () => {
            fireEvent.click(splitBtn);
        });

        await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith('slice_clip', expect.objectContaining({
            trackIndex: 0,
            clipId: 'clip-1',
            samplePos: 0
        })), { timeout: 3000 });
    });

    it('adds a MIDI clip to a track', async () => {
        render(<Timeline />);

        // Wait for tracks to be loaded
        await waitFor(() => expect(screen.getByText('Audio Track 1')).toBeInTheDocument());

        const addMidiBtn = await screen.findByText('M+');

        await act(async () => {
            fireEvent.click(addMidiBtn);
        });

        await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith('add_midi_clip', expect.objectContaining({
            trackIdx: 0,
            clip: expect.objectContaining({
                name: "New MIDI Pattern"
            })
        })));
    });

    it('handles spacebar for play/pause', async () => {
        render(<Timeline />);

        await act(async () => {
            fireEvent.keyDown(window, { code: 'Space' });
        });

        expect(mockInvoke).toHaveBeenCalledWith('is_playing', undefined);
        await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith('play_audio', undefined));
    });

    it('sets up event listeners', async () => {
        render(<Timeline />);

        await waitFor(() => expect(mockListen).toHaveBeenCalledWith('project_updated', expect.any(Function)));
    });
});
