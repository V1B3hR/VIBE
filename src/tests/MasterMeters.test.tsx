import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import { MasterMeters } from '../components/MasterMeters';
import React from 'react';

// Mock Tauri invoke
const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
    invoke: (cmd: string, args: any) => mockInvoke(cmd, args),
}));

describe('MasterMeters', () => {
    beforeEach(() => {
        vi.useFakeTimers();
        mockInvoke.mockReset();
        // Default mock response
        mockInvoke.mockResolvedValue({
            peak_l_db: -60,
            peak_r_db: -60,
            rms_l_db: -60,
            rms_r_db: -60
        });
    });

    afterEach(() => {
        vi.useRealTimers();
        vi.restoreAllMocks();
    });

    it('renders the component title', () => {
        render(<MasterMeters />);
        expect(screen.getByText('MASTER METERS')).toBeInTheDocument();
        expect(screen.getByText('GPU-OFFLOADED')).toBeInTheDocument();
    });


    it('updates meter values based on backend data', async () => {
        // Mock a loud signal
        mockInvoke.mockResolvedValue({
            peak_l_db: -3.0,
            peak_r_db: -6.0,
            rms_l_db: -12.0,
            rms_r_db: -18.0
        });

        render(<MasterMeters />);

        await act(async () => {
            vi.advanceTimersByTime(60);
        });

        // Check if values are rendered in text
        expect(screen.getByText('-3.0 dB')).toBeInTheDocument(); // Peak L
        expect(screen.getByText('RMS: -12.0 dB')).toBeInTheDocument(); // RMS L
    });

    it('handles backend failure gracefully', async () => {
        // Mock a failure
        mockInvoke.mockRejectedValue(new Error('Backend offline'));
        const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => { }); // Suppress error logs

        render(<MasterMeters />);

        await act(async () => {
            vi.advanceTimersByTime(60);
        });

        // Should not crash, displays default or last known values.
        // Defaults are -96.
        expect(screen.getAllByText(/-96.0 dB/).length).toBeGreaterThan(0);

        consoleSpy.mockRestore();
    });

    it('calculates correct width percentages for meter bars', async () => {
        // -48dB should be 50% of the way between -96 and 0
        mockInvoke.mockResolvedValue({
            peak_l_db: -48.0,
            peak_r_db: -48.0,
            rms_l_db: -48.0,
            rms_r_db: -48.0
        });

        const { container } = render(<MasterMeters />);

        await act(async () => {
            vi.advanceTimersByTime(60);
        });

        const meters = container.querySelectorAll('.meter-peak');
        expect(meters.length).toBe(2);

        // Check style width. -48dB + 96 = 48. 48/96 = 0.5 -> 50%
        expect(meters[0]).toHaveStyle('width: 50%');
    });

    it('applies correct color classes based on dB levels', async () => {
        // Clipping signal (> -3dB)
        mockInvoke.mockResolvedValue({
            peak_l_db: -2.0,
            peak_r_db: 0.0,
            rms_l_db: -10.0,
            rms_r_db: -10.0
        });

        const { container } = render(<MasterMeters />);

        await act(async () => {
            vi.advanceTimersByTime(60);
        });

        const meters = container.querySelectorAll('.meter-peak');
        // > -3 should be red
        // The component uses inline styles for color, so we check that
        expect(meters[0]).toHaveStyle('background-color: #ff3333');
    });
});
