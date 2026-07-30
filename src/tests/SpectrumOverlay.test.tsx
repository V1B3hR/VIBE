import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { SpectrumOverlay } from '../components/SpectrumOverlay';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({
    track_a_fft: new Array(128).fill(-30),
    track_b_fft: new Array(128).fill(-30),
    collision_mask: new Array(128).fill(false),
  }),
}));

describe('SpectrumOverlay Component', () => {
  it('renders sidechain spectrum overlay header and legend', async () => {
    render(<SpectrumOverlay trackAIdx={0} trackBIdx={1} trackAName="Kick" trackBName="Bass" />);

    expect(screen.getByText('Sidechain Masking Visualizer')).toBeInTheDocument();
    expect(screen.getByText('Kick')).toBeInTheDocument();
    expect(screen.getByText('Bass')).toBeInTheDocument();
    expect(screen.getByTestId('spectrum-overlay')).toBeInTheDocument();
  });
});
