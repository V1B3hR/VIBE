import React, { useRef, useEffect } from 'react';

interface NanoCompProps {
    threshold: number; // -60 to 0 dB
    ratio: number; // 1.0 to 20.0
    reduction?: number; // 0 to 20 dB (positive value indicating reduction)
    width?: number;
    height?: number;
    onClick?: () => void;
}

export const NanoComp: React.FC<NanoCompProps> = ({ threshold, ratio, reduction = 0, width = 60, height = 40, onClick }) => {
    const canvasRef = useRef<HTMLCanvasElement>(null);

    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        // Clear
        ctx.fillStyle = '#1a1a1a';
        ctx.fillRect(0, 0, width, height);

        // Draw Transfer Curve
        ctx.strokeStyle = '#444';
        ctx.lineWidth = 1;

        ctx.beginPath();
        // 0,0 is bottom-left (-60dB, -60dB)
        // width, height is top-right (0dB, 0dB)

        // Map dB to pixels
        const mapDbX = (db: number) => (db + 60) / 60 * width;
        const mapDbY = (db: number) => height - (db + 60) / 60 * height;

        // Line 1: Linear part (below threshold)
        ctx.moveTo(0, height); // -60, -60
        const threshX = mapDbX(threshold);
        const threshY = mapDbY(threshold);
        ctx.lineTo(threshX, threshY);

        // Line 2: Compressed part (above threshold)
        // At 0dB input: Output = Threshold + (0 - Threshold) / Ratio
        const outAtZero = threshold + (0 - threshold) / ratio;
        ctx.lineTo(width, mapDbY(outAtZero));
        ctx.stroke();

        // Draw Knee Point
        ctx.fillStyle = '#ff9d00';
        ctx.beginPath();
        ctx.arc(threshX, threshY, 2, 0, Math.PI * 2);
        ctx.fill();

        // Draw Gain Reduction (if active)
        if (reduction > 0.1) {
            ctx.fillStyle = 'rgba(255, 50, 50, 0.6)';
            // Draw a downward bar from the top or just a rectangle proportional to GR
            // Let's draw it as a vertical bar on the right side
            const grHeight = (reduction / 20.0) * height; // Map 20dB reduction to full height
            ctx.fillRect(width - 4, 0, 4, grHeight);
        }

        // Border
        ctx.strokeStyle = reduction > 0.1 ? '#882222' : '#333';
        ctx.lineWidth = 1;
        ctx.strokeRect(0, 0, width, height);

    }, [threshold, ratio, reduction, width, height]);

    return (
        <canvas
            ref={canvasRef}
            width={width}
            height={height}
            onClick={onClick}
            style={{ cursor: 'pointer', borderRadius: '4px' }}
        />
    );
};
