import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup, act } from '@testing-library/react';
import { Transport } from './Transport';

// Mock Tauri invoke
const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
    invoke: vi.fn((...args: any[]) => mockInvoke(...args)),
}));

afterEach(() => {
    cleanup();
    vi.useRealTimers();
});

describe('Transport Component', () => {
    beforeEach(() => {
        mockInvoke.mockReset();
        // Default mocks - IMPORTANT: return valid types to prevent component crashes
        mockInvoke.mockImplementation(async (cmd) => {
            if (cmd === 'is_playing') return false;
            if (cmd === 'is_recording') return false;
            if (cmd === 'get_playhead') return 0;
            if (cmd === 'get_bpm') return 120.0;
            if (cmd === 'get_cpu_load') return 5.0;
            return null;
        });
    });

    it('renders and fetches initial state', async () => {
        render(<Transport />);

        // Wait for the polling to call invoke
        await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith('get_bpm'));

        expect(screen.getByTestId('bpm-value')).toBeInTheDocument();
        expect(screen.getByText(/120.0/)).toBeInTheDocument();
    });

    it('toggles play/pause when Play button clicked', async () => {
        render(<Transport />);

        const playBtn = await screen.findByTestId('transport-play');
        fireEvent.click(playBtn);

        expect(mockInvoke).toHaveBeenCalledWith('play_audio');
    });

    it('sends stop commands', async () => {
        render(<Transport />);

        const stopBtn = await screen.findByTestId('transport-stop');

        // Single click: pause
        await act(async () => {
            fireEvent.click(stopBtn);
        });
        expect(mockInvoke).toHaveBeenCalledWith('pause_audio');
    });

    it('updates BPM on double click and input', async () => {
        render(<Transport />);

        const bpmValue = await screen.findByTestId('bpm-value');
        fireEvent.doubleClick(bpmValue);

        // Wait for input to appear
        const bpmInput = await screen.findByDisplayValue('120');
        fireEvent.change(bpmInput, { target: { value: '140' } });

        expect(mockInvoke).toHaveBeenCalledWith('set_bpm', { bpm: 140 });
    });

    it('displays CPU load correctly', async () => {
        mockInvoke.mockImplementation(async (cmd) => {
            if (cmd === 'get_cpu_load') return 85.5;
            if (cmd === 'get_bpm') return 120.0;
            if (cmd === 'is_playing') return false;
            if (cmd === 'is_recording') return false;
            if (cmd === 'get_playhead') return 0;
            return 0;
        });

        render(<Transport />);

        await waitFor(() => {
            const cpuValue = screen.getByTestId('cpu-value');
            expect(cpuValue.textContent).toBe('85.5%');
        });

        const cpuFill = screen.getByTestId('cpu-fill');
        expect(cpuFill).toHaveClass('danger');
    });
    it('toggles metronome when clicked', async () => {
        render(<Transport />);
        const metroBtn = await screen.findByTestId('transport-metronome');
        fireEvent.click(metroBtn);
        // metronomeEnabled becomes true, then calls set_metronome
        expect(mockInvoke).toHaveBeenCalledWith('set_metronome', { enabled: true });
    });

    it('toggles loop when clicked', async () => {
        render(<Transport />);
        const loopBtn = await screen.findByTestId('transport-loop');
        await act(async () => {
            fireEvent.click(loopBtn);
        });
        expect(mockInvoke).toHaveBeenCalledWith('set_loop_enabled', { enabled: true });
    });

    it('toggles record when clicked', async () => {
        render(<Transport />);
        const recBtn = await screen.findByTestId('transport-record');
        await act(async () => {
            fireEvent.click(recBtn);
        });
        expect(mockInvoke).toHaveBeenCalledWith('toggle_record');
    });

    it('handles undo and redo', async () => {
        render(<Transport />);
        const undoBtn = screen.getByTitle(/Undo/i);
        const redoBtn = screen.getByTitle(/Redo/i);

        await act(async () => {
            fireEvent.click(undoBtn);
        });
        expect(mockInvoke).toHaveBeenCalledWith('undo');

        await act(async () => {
            fireEvent.click(redoBtn);
        });
        expect(mockInvoke).toHaveBeenCalledWith('redo');
    });

    it('resets playhead on double-click stop', async () => {
        const dateSpy = vi.spyOn(Date, 'now');
        dateSpy.mockReturnValue(1000); // T=1000

        render(<Transport />);
        const stopBtn = await screen.findByTestId('transport-stop');

        // First click
        await act(async () => {
            fireEvent.click(stopBtn);
        });

        // Advance "time"
        dateSpy.mockReturnValue(1100); // T=1100 (100ms later)

        // Second click within 300ms
        await act(async () => {
            fireEvent.click(stopBtn);
        });

        expect(mockInvoke).toHaveBeenCalledWith('stop_transport');
        expect(mockInvoke).toHaveBeenCalledWith('set_playhead', { sample: 0 });

        dateSpy.mockRestore();
    });

    it('displays RAM usage correctly', async () => {
        mockInvoke.mockImplementation(async (cmd) => {
            if (cmd === 'get_memory_usage') return 45.0;
            if (cmd === 'get_cpu_load') return 5.0;
            if (cmd === 'get_bpm') return 120.0;
            if (cmd === 'is_playing') return false;
            if (cmd === 'is_recording') return false;
            if (cmd === 'get_playhead') return 0;
            return 0;
        });

        render(<Transport />);

        await waitFor(() => {
            const memValue = screen.getByTestId('mem-value');
            expect(memValue.textContent).toBe('45.0%');
        });
    });
});
