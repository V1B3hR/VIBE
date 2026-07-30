import * as React from 'react';
import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './SpatialPanner.css';

interface SurroundGains714 {
  left: number;
  right: number;
  center: number;
  lfe: number;
  left_surround: number;
  right_surround: number;
  left_top_front: number;
  right_top_front: number;
  left_top_back: number;
  right_top_back: number;
}

interface SpatialPannerProps {
  trackName?: string;
  initialX?: number;
  initialY?: number;
  initialZ?: number;
  onPositionChange?: (x: number, y: number, z: number) => void;
}

export const SpatialPanner: React.FC<SpatialPannerProps> = ({
  trackName = 'Track 1',
  initialX = 0.0,
  initialY = 0.0,
  initialZ = 0.0,
  onPositionChange,
}) => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [posX, setPosX] = useState(initialX);
  const [posY, setPosY] = useState(initialY);
  const [posZ, setPosZ] = useState(initialZ);
  const [isDragging, setIsDragging] = useState(false);
  const [gains, setGains] = useState<SurroundGains714 | null>(null);

  useEffect(() => {
    const updateGains = async () => {
      try {
        const res = await invoke<SurroundGains714>('calculate_714_spatial_gains_cmd', {
          x: posX,
          y: posY,
          z: posZ,
        });
        setGains(res);
      } catch (err) {
        console.error('Failed to calculate 3D spatial gains:', err);
      }
    };
    updateGains();
    if (onPositionChange) {
      onPositionChange(posX, posY, posZ);
    }
  }, [posX, posY, posZ, onPositionChange]);

  const handleMouseDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    setIsDragging(true);
    updateCoordsFromMouse(e);
  };

  const handleMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (isDragging) {
      updateCoordsFromMouse(e);
    }
  };

  const handleMouseUp = () => {
    setIsDragging(false);
  };

  const updateCoordsFromMouse = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!canvasRef.current) return;
    const rect = canvasRef.current.getBoundingClientRect();
    const width = rect.width;
    const height = rect.height;

    const mouseX = e.clientX - rect.left;
    const mouseY = e.clientY - rect.top;

    // Map mouse pixel coordinates to Cartesian [-1.0, +1.0]
    const normX = ((mouseX / width) * 2.0 - 1.0).clamp(-1.0, 1.0);
    const normY = ((1.0 - mouseY / height) * 2.0 - 1.0).clamp(-1.0, 1.0);

    setPosX(Number(normX.toFixed(2)));
    setPosY(Number(normY.toFixed(2)));
  };

  useEffect(() => {
    if (!canvasRef.current) return;
    const canvas = canvasRef.current;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const width = canvas.width;
    const height = canvas.height;
    const centerX = width / 2;
    const centerY = height / 2;

    ctx.clearRect(0, 0, width, height);

    // Draw Virtual 3D Room Grid
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.08)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.arc(centerX, centerY, width * 0.42, 0, Math.PI * 2);
    ctx.stroke();

    ctx.beginPath();
    ctx.moveTo(centerX, 0);
    ctx.lineTo(centerX, height);
    ctx.moveTo(0, centerY);
    ctx.lineTo(width, centerY);
    ctx.stroke();

    // Render 7.1.4 Speaker Nodes
    const speakerNodes = [
      { label: 'L', x: centerX - width * 0.35, y: centerY - height * 0.35 },
      { label: 'R', x: centerX + width * 0.35, y: centerY - height * 0.35 },
      { label: 'C', x: centerX, y: centerY - height * 0.42 },
      { label: 'Ls', x: centerX - width * 0.38, y: centerY + height * 0.35 },
      { label: 'Rs', x: centerX + width * 0.38, y: centerY + height * 0.35 },
    ];

    ctx.font = '10px sans-serif';
    ctx.fillStyle = '#666';
    ctx.textAlign = 'center';
    speakerNodes.forEach((node) => {
      ctx.beginPath();
      ctx.arc(node.x, node.y, 6, 0, Math.PI * 2);
      ctx.fillStyle = 'rgba(255, 255, 255, 0.15)';
      ctx.fill();
      ctx.fillStyle = '#aaa';
      ctx.fillText(node.label, node.x, node.y - 10);
    });

    // Render Sound Source Puck
    const puckX = ((posX + 1.0) / 2.0) * width;
    const puckY = ((1.0 - posY) / 2.0) * height;

    // Glowing distance ring
    ctx.beginPath();
    ctx.arc(puckX, puckY, 18, 0, Math.PI * 2);
    ctx.fillStyle = 'rgba(112, 0, 255, 0.25)';
    ctx.fill();

    // Core Puck
    ctx.beginPath();
    ctx.arc(puckX, puckY, 8, 0, Math.PI * 2);
    ctx.fillStyle = '#00f3ff';
    ctx.shadowColor = '#00f3ff';
    ctx.shadowBlur = 12;
    ctx.fill();
    ctx.shadowBlur = 0;

    // Label
    ctx.fillStyle = '#fff';
    ctx.font = 'bold 11px sans-serif';
    ctx.fillText(trackName, puckX, puckY + 22);
  }, [posX, posY, posZ, trackName]);

  return (
    <div className="spatial-panner-container" data-testid="spatial-panner">
      <div className="spatial-panner-header">
        <span>3D VBAP Spatial Panner (7.1.4 / Binaural)</span>
        <div className="spatial-coords-readout">
          X: {posX > 0 ? `+${posX}` : posX} | Y: {posY > 0 ? `+${posY}` : posY} | Z: {posZ > 0 ? `+${posZ}` : posZ}
        </div>
      </div>
      <canvas
        ref={canvasRef}
        className="spatial-panner-canvas"
        width={600}
        height={200}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
      />
    </div>
  );
};

declare global {
  interface Number {
    clamp(min: number, max: number): number;
  }
}

if (!Number.prototype.clamp) {
  Number.prototype.clamp = function (min: number, max: number) {
    return Math.min(Math.max(this.valueOf(), min), max);
  };
}
