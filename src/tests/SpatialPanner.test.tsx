import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { SpatialPanner } from '../components/SpatialPanner';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({
    left: 0.5,
    right: 0.5,
    center: 0.0,
    lfe: 0.1,
    left_surround: 0.2,
    right_surround: 0.2,
    left_top_front: 0.1,
    right_top_front: 0.1,
    left_top_back: 0.1,
    right_top_back: 0.1,
  }),
}));

describe('SpatialPanner Component', () => {
  it('renders 3D spatial panner header and coordinates readout', () => {
    render(<SpatialPanner trackName="Vocal Lead" initialX={0.2} initialY={0.5} />);

    expect(screen.getByText('3D VBAP Spatial Panner (7.1.4 / Binaural)')).toBeInTheDocument();
    expect(screen.getByText(/X: \+0.2 \| Y: \+0.5 \| Z: 0/)).toBeInTheDocument();
    expect(screen.getByTestId('spatial-panner')).toBeInTheDocument();
  });
});
