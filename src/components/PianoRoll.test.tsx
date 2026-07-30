import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup, act } from '@testing-library/react';
import { PianoRoll } from './PianoRoll';

// Mock Tauri
const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
    invoke: vi.fn((cmd: string, args: any) => mockInvoke(cmd, args)),
}));

// Mock child components
vi.mock('./ArpeggiatorModal', () => ({ ArpeggiatorModal: () => <div data-testid="mock-arpeggiator-modal" /> }));

// Prevent Tauri internals error
(global as any).window.__TAURI_INTERNALS__ = {
    transformCallback: vi.fn(),
    invoke: vi.fn(),
    metadata: {}
};

describe('PianoRoll Component', () => {
    const mockMidiClip = {
        id: 'clip-1',
        name: 'Lead Pattern',
        start_sample: 0,
        length_samples: 48000 * 4,
        notes: [
            { start_sample: 0, length_samples: 4800, note: 60, velocity: 100, channel: 0, probability: 1.0, velocity_random: 0, timing_random: 0 }
        ],
        color: '#00ff00',
        tuning_steps: 12
    };

    beforeEach(() => {
        vi.clearAllMocks();

        mockInvoke.mockImplementation(async (cmd, args) => {
            if (cmd === 'get_midi_clip_data') return mockMidiClip;
            if (cmd === 'get_track_midi_clips') return [mockMidiClip];
            if (cmd === 'get_tracks') return [];
            if (cmd === 'is_playing') return false;
            return null;
        });

        // Mock Canvas
        HTMLCanvasElement.prototype.getContext = vi.fn().mockReturnValue({
            clearRect: vi.fn(),
            fillRect: vi.fn(),
            beginPath: vi.fn(),
            moveTo: vi.fn(),
            lineTo: vi.fn(),
            stroke: vi.fn(),
            fill: vi.fn(),
            rect: vi.fn(),
            strokeRect: vi.fn(),
            fillText: vi.fn(),
            setLineDash: vi.fn(),
            measureText: vi.fn().mockReturnValue({ width: 0 }),
        });

        // Mock ResizeObserver
        global.ResizeObserver = class ResizeObserver {
            observe() { }
            unobserve() { }
            disconnect() { }
        };
    });

    afterEach(cleanup);

    it('renders and loads clip data', async () => {
        render(<PianoRoll trackIdx={0} clipId="clip-1" onClose={() => { }} />);

        await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith('get_midi_clip_data', expect.objectContaining({ clipId: 'clip-1' })));
        expect(screen.getByTestId('piano-roll-canvas')).toBeInTheDocument();
    });

    it('switches tools correctly', async () => {
        render(<PianoRoll trackIdx={0} clipId="clip-1" onClose={() => { }} />);

        const pencilTool = await screen.findByTestId('tool-pencil-btn');
        fireEvent.click(pencilTool);
        expect(pencilTool).toHaveClass('active');

        const brushTool = screen.getByTestId('tool-brush-btn');
        fireEvent.click(brushTool);
        expect(brushTool).toHaveClass('active');
    });

    it('handles snap change', async () => {
        render(<PianoRoll trackIdx={0} clipId="clip-1" onClose={() => { }} />);

        const snapSelect = await screen.findByTestId('piano-roll-snap-select');
        fireEvent.change(snapSelect, { target: { value: '8' } });
        expect(snapSelect).toHaveValue('8');
    });

    it('toggles macro recording', async () => {
        render(<PianoRoll trackIdx={0} clipId="clip-1" onClose={() => { }} />);

        const recordBtn = await screen.findByTestId('piano-roll-macro-record-btn');
        fireEvent.click(recordBtn);
        expect(recordBtn).toHaveClass('recording-btn');

        fireEvent.click(recordBtn);
        expect(recordBtn).not.toHaveClass('recording-btn');
    });

    it('calls quantize on button click', async () => {
        render(<PianoRoll trackIdx={0} clipId="clip-1" onClose={() => { }} />);

        const quantizeBtn = await screen.findByTestId('piano-roll-quantize-btn');
        await act(async () => {
            fireEvent.click(quantizeBtn);
        });

        expect(mockInvoke).toHaveBeenCalledWith('quantize_notes', expect.objectContaining({
            trackIdx: 0,
            clipId: 'clip-1',
            division: 'Sixteenth'
        }));
    });

    it('calls play/pause on spacebar', async () => {
        render(<PianoRoll trackIdx={0} clipId="clip-1" onClose={() => { }} />);

        // Ensure clip is loaded and component re-rendered with new listener
        await waitFor(() => {
            const status = screen.queryByText(/NOTES:/);
            return expect(status?.parentElement?.textContent).toContain('NOTES: 1');
        });

        await act(async () => {
            fireEvent.keyDown(window, { key: ' ', code: 'Space' });
        });

        expect(mockInvoke).toHaveBeenCalledWith('is_playing', undefined);
        await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith('play_audio', undefined));
    });

    it('closes on finish editing', async () => {
        const onClose = vi.fn();
        render(<PianoRoll trackIdx={0} clipId="clip-1" onClose={onClose} />);

        const closeBtn = await screen.findByTestId('piano-roll-close-btn');
        fireEvent.click(closeBtn);

        expect(onClose).toHaveBeenCalled();
    });
});
