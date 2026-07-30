import React, { useRef, useEffect } from 'react';

interface AdsrGraphProps {
    attack: number;
    decay: number;
    sustain: number;
    release: number;
    onParamChange: (param: 'A' | 'D' | 'S' | 'R', value: number) => void;
    color?: string;
    width?: number;
    height?: number;
}

export const AdsrGraph: React.FC<AdsrGraphProps> = ({
    attack, decay, sustain, release, onParamChange,
    color = '#00ffed',
    width = 240,
    height = 100
}) => {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const isDragging = useRef<'A' | 'D' | 'S' | 'R' | null>(null);

    const totalDuration = Math.max(attack + decay + 0.5 + release, 1.0);
    const xScale = width / totalDuration;

    const getPoints = () => {
        return {
            A: { x: attack * xScale, y: 0 },
            D: { x: (attack + decay) * xScale, y: height - (sustain * height) },
            S: { x: (attack + decay + 0.5) * xScale, y: height - (sustain * height) },
            R: { x: (attack + decay + 0.5 + release) * xScale, y: height }
        };
    };

    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        ctx.clearRect(0, 0, width, height);

        // Styling
        ctx.strokeStyle = color;
        ctx.lineWidth = 2;
        ctx.fillStyle = color + '22';

        const pts = getPoints();

        ctx.beginPath();
        ctx.moveTo(0, height);
        ctx.lineTo(pts.A.x, pts.A.y);
        ctx.lineTo(pts.D.x, pts.D.y);
        ctx.lineTo(pts.S.x, pts.S.y);
        ctx.lineTo(pts.R.x, pts.R.y);
        ctx.stroke();

        ctx.lineTo(0, height);
        ctx.closePath();
        ctx.fill();

        // Dots
        ctx.fillStyle = '#fff';
        const drawDot = (p: { x: number, y: number }) => {
            ctx.beginPath();
            ctx.arc(p.x, p.y, 4, 0, Math.PI * 2);
            ctx.fill();
            ctx.strokeStyle = color;
            ctx.lineWidth = 1;
            ctx.stroke();
        };

        drawDot(pts.A);
        drawDot(pts.D);
        drawDot(pts.S);
        drawDot(pts.R);

    }, [attack, decay, sustain, release, color, width, height]);

    const handleMouseDown = (e: React.MouseEvent) => {
        const rect = canvasRef.current?.getBoundingClientRect();
        if (!rect) return;
        const mx = e.clientX - rect.left;
        const my = e.clientY - rect.top;

        const pts = getPoints();
        const dist = (p: { x: number, y: number }) => Math.sqrt((p.x - mx) ** 2 + (p.y - my) ** 2);

        if (dist(pts.A) < 15) isDragging.current = 'A';
        else if (dist(pts.D) < 15) isDragging.current = 'D';
        else if (dist(pts.S) < 15) isDragging.current = 'S';
        else if (dist(pts.R) < 15) isDragging.current = 'R';
    };

    const handleMouseMove = (e: React.MouseEvent) => {
        if (!isDragging.current) return;
        const rect = canvasRef.current?.getBoundingClientRect();
        if (!rect) return;
        const mx = e.clientX - rect.left;
        const my = e.clientY - rect.top;

        const valX = mx / xScale;
        const valY = 1.0 - (my / height);

        if (isDragging.current === 'A') {
            onParamChange('A', Math.max(0.001, Math.min(5.0, valX)));
        } else if (isDragging.current === 'D') {
            onParamChange('D', Math.max(0.001, Math.min(5.0, valX - attack)));
        } else if (isDragging.current === 'S') {
            onParamChange('S', Math.max(0, Math.min(1.0, valY)));
        } else if (isDragging.current === 'R') {
            onParamChange('R', Math.max(0.001, Math.min(5.0, valX - (attack + decay + 0.5))));
        }
    };

    const handleMouseUp = () => {
        isDragging.current = null;
    };

    return (
        <canvas
            ref={canvasRef}
            width={width}
            height={height}
            onMouseDown={handleMouseDown}
            onMouseMove={handleMouseMove}
            onMouseUp={handleMouseUp}
            onMouseLeave={handleMouseUp}
            style={{ background: '#080808', borderRadius: '4px', border: '1px solid #222', cursor: isDragging.current ? 'grabbing' : 'crosshair' }}
        />
    );
};
