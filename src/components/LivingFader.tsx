import * as React from 'react';
import { useRef, useEffect, useState } from 'react';

interface LivingFaderProps {
    value: number; // Volume dB (-60 to +6) or linear? Mixer uses Parameter (dB or linear?)
    // Track.volume is configured -60 to +6 dB in graph.rs.
    // So value is dB.
    onChange: (val: number) => void;

    // Meters (dB)
    peakL: number;
    peakR: number;
    lufsM: number; // Momentary LUFS
    truePeakL: number;
    truePeakR: number;

    height?: number; // 300px default
}

export const LivingFader: React.FC<LivingFaderProps> = ({ value, onChange, peakL, peakR, lufsM, truePeakL, truePeakR, height = 300 }) => {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const [isDragging, setIsDragging] = useState(false);

    const startY = useRef(0);
    const startVal = useRef(0);

    // Ghost Meter State
    const lastPeakL = useRef(-144);
    const lastPeakR = useRef(-144);
    const ghostRefL = useRef({ val: -144, time: 0 });
    const ghostRefR = useRef({ val: -144, time: 0 });

    const width = 60; // Fixed width for component
    const capHeight = 40;

    // Helper: dB to Y pixel
    // Range: +6dB (top) to -60dB (bottom). Total 66dB.
    const mapDbToY = (db: number) => {
        const topDb = 6;
        const bottomDb = -60;
        const range = topDb - bottomDb;
        const pct = (db - bottomDb) / range;
        return height - (pct * height); // Inverted Y
    };

    const mapYToDb = (y: number) => {
        const topDb = 6;
        const bottomDb = -60;
        const range = topDb - bottomDb;
        const pct = 1.0 - (y / height);
        return bottomDb + (pct * range);
    };

    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        // Animate loop for metering
        // In React, we rely on Props updating. If props update at 60fps, this runs 60fps.
        // If props update slower (10fps), visual interpolation would be needed for smooth meters.
        // For now, draw immediately.

        ctx.clearRect(0, 0, width, height);

        // Track (Slot)
        const slotX = width / 2 - 2;
        ctx.fillStyle = '#111';
        ctx.fillRect(slotX, 0, 4, height);
        ctx.fillStyle = '#222';
        ctx.fillRect(slotX + 1, 0, 2, height);

        // --- METERS ---
        // Draw Meters BEHIND the fader cap or Next to track?
        // Plan: "Inline... combined". Let's put them on sides of the slot.
        // L on left, R on right.

        // Update Ghost
        const now = Date.now();
        if (peakL > ghostRefL.current.val) {
            ghostRefL.current = { val: peakL, time: now };
        } else if (now - ghostRefL.current.time > 3000) {
            // Decay
            ghostRefL.current.val -= 0.5; // slow decay
        }
        if (peakR > ghostRefR.current.val) {
            ghostRefR.current = { val: peakR, time: now };
        } else if (now - ghostRefR.current.time > 3000) {
            // Decay
            ghostRefR.current.val -= 0.5;
        }

        const drawMeter = (val: number, x: number, w: number, isR: boolean, lufsVal: number, ghostVal: number) => {
            // Background
            ctx.fillStyle = '#0a0a0a';
            ctx.fillRect(x, 0, w, height);

            // LUFS (Darker/Background Bar)
            // Clamped -60 to +6
            const lufsY = Math.max(0, mapDbToY(lufsVal));
            const fillH_Lufs = height - lufsY;
            ctx.fillStyle = '#005577'; // Deep Blue for LUFS
            ctx.fillRect(x, lufsY, w, fillH_Lufs);

            // Peak (Bright Bar)
            const peakY = Math.max(0, mapDbToY(val));
            const fillH = height - peakY;

            // Gradient
            const grad = ctx.createLinearGradient(0, height, 0, 0);
            grad.addColorStop(0, '#00ff00'); // -60
            grad.addColorStop(0.7, '#ffff00'); // -10
            grad.addColorStop(0.9, '#ff0000'); // 0
            grad.addColorStop(1, '#ffffff'); // +6

            ctx.fillStyle = grad;
            ctx.fillRect(x, peakY, w, fillH);

            // Ghost Line
            const ghostY = Math.max(0, mapDbToY(ghostVal));
            ctx.fillStyle = '#ffe500';
            ctx.fillRect(x, ghostY, w, 2);

            // Clip Indicator
            if (val > 0) {
                ctx.fillStyle = '#ff0000';
                ctx.shadowColor = '#ff0000';
                ctx.shadowBlur = 10;
                ctx.fillRect(x, 0, w, 5);
                ctx.shadowBlur = 0;
            }
        };

        // Left Meter
        drawMeter(peakL, 8, 6, false, lufsM, ghostRefL.current.val);
        // Right Meter
        drawMeter(peakR, width - 14, 6, true, lufsM, ghostRefR.current.val);

        // Draw True Peak Indicators (Small red lines)
        const tpLY = mapDbToY(truePeakL);
        const tpRY = mapDbToY(truePeakR);
        ctx.fillStyle = truePeakL > 0 ? '#ff00ff' : '#ff4444';
        ctx.fillRect(8, tpLY, 6, 2);
        ctx.fillStyle = truePeakR > 0 ? '#ff00ff' : '#ff4444';
        ctx.fillRect(width - 14, tpRY, 6, 2);


        // --- FADER CAP ---
        const capY = Math.max(0, Math.min(height - capHeight, mapDbToY(value) - capHeight / 2));

        ctx.shadowColor = 'rgba(0,0,0,0.5)';
        ctx.shadowBlur = 5;
        ctx.fillStyle = '#333';
        // Fader Cap Body
        ctx.fillRect(4, capY, width - 8, capHeight);

        // Cyberpunk Glow / Pulse based on Signal
        // Max signal (PeakL/R) determines pulse intensity
        const maxPeak = Math.max(peakL, peakR);
        // Map -60 to 0 -> 0 to 1 intensity
        let pulse = 0;
        if (maxPeak > -60) {
            pulse = (maxPeak + 60) / 60;
            pulse = Math.max(0, Math.min(1, pulse));
        }

        // Color shifts from Green -> Yellow -> Red
        let pulseColor = `rgba(0, 255, 0, ${pulse * 0.8})`;
        if (maxPeak > -6) pulseColor = `rgba(255, 200, 0, ${pulse})`; // Yellowish
        if (maxPeak > 0) pulseColor = `rgba(255, 0, 0, ${pulse})`; // Red

        // LED Strip on Cap
        ctx.fillStyle = '#111';
        ctx.fillRect(6, capY + capHeight / 2 - 2, width - 12, 4); // Groove

        ctx.shadowColor = pulseColor;
        ctx.shadowBlur = 10 * pulse;
        ctx.fillStyle = pulseColor;
        ctx.fillRect(8, capY + capHeight / 2 - 1, width - 16, 2); // LED
        ctx.shadowBlur = 0;

        // DB Readout on hover or always?
        if (isDragging) {
            ctx.fillStyle = '#fff';
            ctx.font = '10px Inter';
            ctx.textAlign = 'center';
            ctx.fillText((value ?? 0).toFixed(1), width / 2, capY - 5);
        } else {
            // Draw 0dB line
            const zeroY = mapDbToY(0);
            ctx.strokeStyle = '#666';
            ctx.lineWidth = 1;
            ctx.beginPath();
            ctx.moveTo(0, zeroY); ctx.lineTo(width, zeroY);
            ctx.stroke();
        }

    }, [value, peakL, peakR, lufsM, truePeakL, truePeakR, height, isDragging]);

    const handleMouseDown = (e: React.MouseEvent) => {
        setIsDragging(true);
        startY.current = e.clientY;
        startVal.current = value; // Store current dB

        document.body.style.cursor = 'ns-resize';
        window.addEventListener('mousemove', handleMouseMove);
        window.addEventListener('mouseup', handleMouseUp);
    };

    const handleMouseMove = (e: MouseEvent) => {
        const dy = startY.current - e.clientY;
        // Sensitivity: 300px = 66dB -> 1px = 0.22dB
        const pxToDb = 66 / height;
        const dbChange = dy * pxToDb;

        let newVal = startVal.current + dbChange;
        newVal = Math.max(-60, Math.min(6, newVal));
        onChange(newVal);
    };

    const handleMouseUp = () => {
        setIsDragging(false);
        document.body.style.cursor = 'default';
        window.removeEventListener('mousemove', handleMouseMove);
        window.removeEventListener('mouseup', handleMouseUp);
    };

    // Double click to reset
    const handleDoubleClick = () => {
        onChange(0.0);
    };

    return (
        <canvas
            ref={canvasRef}
            width={width}
            height={height}
            onMouseDown={handleMouseDown}
            onDoubleClick={handleDoubleClick}
            style={{
                cursor: 'ns-resize',
                borderRadius: '4px',
                background: '#050505',
                border: '1px solid #222'
            }}
        />
    );
};
