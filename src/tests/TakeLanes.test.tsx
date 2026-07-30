import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { TakeLanes } from '../components/TakeLanes';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('TakeLanes Component', () => {
  const mockTakeLanes = [
    { id: 'lane-1', name: 'Take 1', is_muted: false, is_solo: false },
    { id: 'lane-2', name: 'Take 2', is_muted: true, is_solo: false },
  ];

  const mockCompRegions = [
    { id: 'reg-1', take_lane_id: 'lane-1', start_sample: 0, end_sample: 1000 },
  ];

  it('renders take lane headers and sub-lane rows', () => {
    render(
      <TakeLanes
        trackId="track-1"
        takeLanes={mockTakeLanes}
        compRegions={mockCompRegions}
        pixelsPerSample={0.1}
        onCompRegionsChange={() => {}}
      />
    );

    expect(screen.getByText('Take 1')).toBeInTheDocument();
    expect(screen.getByText('Take 2')).toBeInTheDocument();
    expect(screen.getByTestId('take-lanes-container')).toBeInTheDocument();
  });
});
