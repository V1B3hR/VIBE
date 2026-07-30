// ClipMatrix.tsx - Dual-View Non-Linear Clip Launcher Grid Component
// Renders Ableton Live style matrix for live clip auditioning and performance loop triggering.

import React, { useState } from 'react';
import './ClipMatrix.css';

export interface MatrixClip {
  id: string;
  name: string;
  color?: string;
  isPlaying: boolean;
  isQueued: boolean;
}

export interface MatrixTrack {
  id: number;
  name: string;
  clips: (MatrixClip | null)[];
}

export interface ClipMatrixProps {
  tracks?: MatrixTrack[];
  onLaunchClip?: (trackId: number, sceneIndex: number) => void;
  onStopTrack?: (trackId: number) => void;
}

export const ClipMatrix: React.FC<ClipMatrixProps> = ({
  tracks = [
    {
      id: 1,
      name: 'Drums',
      clips: [
        { id: 'c1', name: '808 Beat A', isPlaying: true, isQueued: false },
        { id: 'c2', name: '808 Fill B', isPlaying: false, isQueued: false },
        null,
      ],
    },
    {
      id: 2,
      name: 'Bass',
      clips: [
        { id: 'c3', name: 'Sub Bass 1', isPlaying: true, isQueued: false },
        { id: 'c4', name: 'Slap Synth', isPlaying: false, isQueued: true },
        null,
      ],
    },
    {
      id: 3,
      name: 'Synth Lead',
      clips: [
        { id: 'c5', name: 'Arp Cyber', isPlaying: false, isQueued: false },
        { id: 'c6', name: 'Neon Hook', isPlaying: false, isQueued: false },
        null,
      ],
    },
  ],
  onLaunchClip,
  onStopTrack,
}) => {
  const [selectedScene, setSelectedScene] = useState<number | null>(null);

  const sceneCount = Math.max(...tracks.map((t) => t.clips.length), 3);

  return (
    <div className="clip-matrix-container">
      <div className="clip-matrix-header">
        <span className="clip-matrix-title">🎛️ Session Clip Matrix</span>
        <div style={{ display: 'flex', gap: '8px' }}>
          <button
            style={{
              padding: '4px 10px',
              fontSize: '11px',
              borderRadius: '4px',
              border: '1px solid rgba(255,255,255,0.2)',
              background: 'rgba(255,255,255,0.05)',
              color: '#fff',
              cursor: 'pointer',
            }}
            onClick={() => setSelectedScene(0)}
          >
            Launch Scene 1
          </button>
        </div>
      </div>

      <div className="clip-matrix-grid">
        {tracks.map((track) => (
          <div key={track.id} className="clip-matrix-column">
            <div className="clip-column-header">{track.name}</div>
            {Array.from({ length: sceneCount }).map((_, sceneIdx) => {
              const clip = track.clips[sceneIdx];
              return (
                <div
                  key={sceneIdx}
                  className={`clip-cell ${clip?.isPlaying ? 'is-playing' : ''} ${
                    clip?.isQueued ? 'is-queued' : ''
                  }`}
                  onClick={() => onLaunchClip && onLaunchClip(track.id, sceneIdx)}
                >
                  <span className="clip-cell-name">
                    {clip ? clip.name : '— Stop —'}
                  </span>
                </div>
              );
            })}
          </div>
        ))}
      </div>
    </div>
  );
};

export default ClipMatrix;
