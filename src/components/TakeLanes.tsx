import * as React from 'react';
import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './TakeLanes.css';

export interface TakeLaneData {
  id: string;
  name: string;
  is_muted: boolean;
  is_solo: boolean;
}

export interface CompRegionData {
  id: string;
  take_lane_id: string;
  start_sample: number;
  end_sample: number;
}

interface TakeLanesProps {
  trackId: string;
  takeLanes: TakeLaneData[];
  compRegions: CompRegionData[];
  pixelsPerSample: number;
  onCompRegionsChange: (updated: CompRegionData[]) => void;
}

export const TakeLanes: React.FC<TakeLanesProps> = ({
  trackId,
  takeLanes,
  compRegions,
  pixelsPerSample,
  onCompRegionsChange,
}) => {
  const [isSwiping, setIsSwiping] = useState(false);
  const [swipeStart, setSwipeStart] = useState<number | null>(null);

  const handleMouseDown = (laneId: string, e: React.MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const startSample = Math.round(x / pixelsPerSample);

    setIsSwiping(true);
    setSwipeStart(startSample);
  };

  const handleMouseUp = async (laneId: string, e: React.MouseEvent<HTMLDivElement>) => {
    if (!isSwiping || swipeStart === null) return;

    const rect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const endSample = Math.round(x / pixelsPerSample);

    const start = Math.min(swipeStart, endSample);
    const end = Math.max(swipeStart, endSample);

    if (end - start > 100) {
      try {
        const updated = await invoke<CompRegionData[]>('select_comp_region', {
          existingRegions: compRegions,
          takeLaneIdStr: laneId,
          startSample: start,
          endSample: end,
        });
        onCompRegionsChange(updated);
      } catch (err) {
        console.error('Failed to update comp region:', err);
      }
    }

    setIsSwiping(false);
    setSwipeStart(null);
  };

  return (
    <div className="take-lanes-container" data-testid="take-lanes-container">
      {takeLanes.map((lane, idx) => {
        const laneRegions = compRegions.filter((r) => r.take_lane_id === lane.id);

        return (
          <div key={lane.id || idx} className="take-lane-row">
            <div className="take-lane-header">
              <span className="take-lane-name">{lane.name || `Take ${idx + 1}`}</span>
              <button
                className={`take-lane-btn ${lane.is_muted ? 'active' : ''}`}
                title="Mute Lane"
              >
                M
              </button>
            </div>
            <div
              className="take-lane-track"
              onMouseDown={(e) => handleMouseDown(lane.id, e)}
              onMouseUp={(e) => handleMouseUp(lane.id, e)}
            >
              {laneRegions.map((region) => {
                const left = region.start_sample * pixelsPerSample;
                const width = (region.end_sample - region.start_sample) * pixelsPerSample;

                return (
                  <div
                    key={region.id}
                    className="comp-region-highlight"
                    style={{ left: `${left}px`, width: `${width}px` }}
                    title={`Comp Region: ${region.start_sample} - ${region.end_sample}`}
                  />
                );
              })}
            </div>
          </div>
        );
      })}
    </div>
  );
};
