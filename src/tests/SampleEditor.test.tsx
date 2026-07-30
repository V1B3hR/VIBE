import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { SampleEditor } from '../components/SampleEditor';
import React from 'react';

// Mock Tauri invoke
const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
    invoke: (cmd: string, args: any) => mockInvoke(cmd, args),
}));

// Mock child components
vi.mock('../components/Waveform', () => ({ Waveform: () => <div data-testid="waveform" /> }));
vi.mock('../components/MelSpectrogram', () => ({ MelSpectrogram: () => <div data-testid="spectrogram" /> }));

describe('SampleEditor', () => {
    const mockClip = {
        id: 'clip-1',
        name: 'Thunder Clap',
        peaks: [[0.1, 0.5, 0.2]],
        trackIndex: 0,
        start_sample: 0,
        duration_samples: 44100,
    };

    beforeEach(() => {
        mockInvoke.mockReset();
    });

    it('renders waveform by default', () => {
        render(<SampleEditor clip={mockClip} onClose={() => { }} />);
        expect(screen.getByText('VIBE PRO SAMPLER / Thunder Clap')).toBeInTheDocument();
        expect(screen.getByTestId('waveform')).toBeInTheDocument();
    });

    it('switches to spectrogram and triggers analysis', async () => {
        mockInvoke.mockResolvedValue({ frames: [] });
        render(<SampleEditor clip={mockClip} onClose={() => { }} />);

        const specBtn = screen.getByText('Spectrogram');
        fireEvent.click(specBtn);

        expect(screen.getByText('Spectrogram')).toHaveClass('active');
        expect(mockInvoke).toHaveBeenCalledWith('analyze_spectral', {
            trackIdx: 0,
            clipId: 'clip-1',
        });

        await waitFor(() => expect(screen.getByTestId('spectrogram')).toBeInTheDocument());
    });

    it('calls onClose when close button is clicked', () => {
        const onClose = vi.fn();
        render(<SampleEditor clip={mockClip} onClose={onClose} />);

        const closeBtn = screen.getByText('×');
        fireEvent.click(closeBtn);

        expect(onClose).toHaveBeenCalled();
    });

    it('renders processing and fade UI elements', () => {
        render(<SampleEditor clip={mockClip} onClose={() => { }} />);

        expect(screen.getByText('NORMALIZE PEAKS')).toBeInTheDocument();
        expect(screen.getByText('REVERSE & WARP')).toBeInTheDocument();
        expect(screen.getByText('CROP TO PERFECT LOOP')).toBeInTheDocument();
        expect(screen.getByText('Attack')).toBeInTheDocument();
        expect(screen.getByText('Release')).toBeInTheDocument();
    });
});
