import * as React from 'react';
import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './SpectrumOverlay.css';

interface SidechainMaskingFrame {
  track_a_fft: number[];
  track_b_fft: number[];
  collision_mask: boolean[];
}

interface SpectrumOverlayProps {
  trackAIdx: number;
  trackBIdx: number;
  trackAName?: string;
  trackBName?: string;
}

export const SpectrumOverlay: React.FC<SpectrumOverlayProps> = ({
  trackAIdx,
  trackBIdx,
  trackAName = 'Track A',
  trackBName = 'Sidechain (Track B)',
}) => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [frameData, setFrameData] = useState<SidechainMaskingFrame | null>(null);

  useEffect(() => {
    let isMounted = true;
    const fetchSpectrum = async () => {
      try {
        const frame = await invoke<SidechainMaskingFrame>('get_sidechain_spectrum_comparison', {
          trackAIdx,
          trackBIdx,
        });
        if (isMounted) {
          setFrameData(frame);
        }
      } catch (err) {
        console.error('Failed to fetch sidechain spectrum comparison:', err);
      }
    };

    fetchSpectrum();
    const interval = setInterval(fetchSpectrum, 50); // 20Hz polling

    return () => {
      isMounted = false;
      clearInterval(interval);
    };
  }, [trackAIdx, trackBIdx]);

  useEffect(() => {
    if (!canvasRef.current || !frameData) return;
    const canvas = canvasRef.current;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const width = canvas.width;
    const height = canvas.height;

    ctx.clearRect(0, 0, width, height);

    // Draw Grid Lines
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.05)';
    ctx.lineWidth = 1;
    for (let y = 0; y < height; y += 40) {
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(width, y);
      ctx.stroke();
    }

    const bands = frameData.track_a_fft.length;
    const bandWidth = width / bands;

    // Draw Collision Warning Highlights
    ctx.fillStyle = 'rgba(255, 170, 0, 0.25)';
    for (let i = 0; i < bands; i++) {
      if (frameData.collision_mask[i]) {
        const x = i * bandWidth;
        ctx.fillRect(x, 0, bandWidth, height);
      }
    }

    // Function to map dB (-60dB to 0dB) to Y coordinate
    const dbToY = (db: number) => {
      const clamped = Math.max(-60, Math.min(0, db));
      return height - ((clamped + 60) / 60) * height;
    };

    // Render Curve for Track A (Cyan)
    ctx.beginPath();
    ctx.strokeStyle = '#00f3ff';
    ctx.lineWidth = 2;
    for (let i = 0; i < bands; i++) {
      const x = i * bandWidth + bandWidth / 2;
      const y = dbToY(frameData.track_a_fft[i]);
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.stroke();

    // Render Curve for Track B (Magenta)
    ctx.beginPath();
    ctx.strokeStyle = '#ff0055';
    ctx.lineWidth = 2;
    for (let i = 0; i < bands; i++) {
      const x = i * bandWidth + bandWidth / 2;
      const y = dbToY(frameData.track_b_fft[i]);
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.stroke();
  }, [frameData]);

  return (
    <div className="spectrum-overlay-container" data-testid="spectrum-overlay">
      <div className="spectrum-overlay-header">
        <span>Sidechain Masking Visualizer</span>
        <div className="spectrum-legend">
          <div className="legend-item">
            <span className="legend-color track-a" />
            <span>{trackAName}</span>
          </div>
          <div className="legend-item">
            <span className="legend-color track-b" />
            <span>{trackBName}</span>
          </div>
          <div className="legend-item">
            <span className="legend-color collision" />
            <span>Collision Zone</span>
          </div>
        </div>
      </div>
      <canvas ref={canvasRef} className="spectrum-canvas" width={800} height={160} />
    </div>
  );
};
