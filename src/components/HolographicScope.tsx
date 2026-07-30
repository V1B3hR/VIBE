import React, { useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './HolographicScope.css';

export const HolographicScope: React.FC = () => {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const frameId = useRef<number | null>(null);

    useEffect(() => {
        const fetchAndDraw = async () => {
            try {
                // Returns tuple [left_channel, right_channel]
                const data = await invoke<[number[], number[]]>('get_scope_data');
                if (data && data.length === 2) {
                    drawScope(data[0], data[1]);
                }
            } catch (e) {
                // console.error(e);
            }
            frameId.current = requestAnimationFrame(fetchAndDraw);
        };

        frameId.current = requestAnimationFrame(fetchAndDraw);

        return () => {
            if (frameId.current) cancelAnimationFrame(frameId.current);
        };
    }, []);

    const drawScope = (l: number[], r: number[]) => {
        const canvas = canvasRef.current;
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        const w = canvas.width;
        const h = canvas.height;
        const cx = w / 2;
        const cy = h / 2;

        // Phosphor Persistence (Fade out)
        ctx.fillStyle = "rgba(10, 10, 16, 0.25)";
        ctx.fillRect(0, 0, w, h);

        ctx.lineWidth = 1.5;
        ctx.lineJoin = "round";
        ctx.globalCompositeOperation = "lighter";

        // Draw Lissajous (X = L, Y = R) purely
        // Or: 
        //   X = L + MonoOffset?
        //   Y = R + MonoOffset?
        // Standard Goniometer (Vector Scope):
        //   X = Side (L - R)
        //   Y = Mid  (L + R) 
        // Or simple L vs R rotated 45deg.

        // Let's do Standard Lissajous: L on X, R on Y
        // But rotated 45 degrees usually looks best for stereo field (Goniometer style)
        // Goniometer:
        // x = (L - R) * scale + cx
        // y = (L + R) * scale + cy (inverted y usually)

        ctx.beginPath();

        // Color Cycle or just Cyan?
        ctx.strokeStyle = "#00ffed";
        ctx.shadowColor = "#00ffed";
        ctx.shadowBlur = 10;

        const scale = h * 0.4;

        // We limit points to prevent choking, but backend sends 2048. 
        // Drawing 2048 segments is fine for canvas.

        for (let i = 0; i < l.length; i += 2) { // step 2 for speed
            const left = l[i];
            const right = r[i];

            // Goniometer Transform
            // Mid (Vertical): L+R
            // Side (Horizontal): L-R

            const mid = (left + right) * 0.707;
            const side = (left - right) * 0.707;

            const x = cx + side * scale;
            const y = cy - mid * scale; // -mid because Y is down

            if (i === 0) {
                ctx.moveTo(x, y);
            } else {
                ctx.lineTo(x, y);
            }
        }
        ctx.stroke();

        ctx.globalCompositeOperation = "source-over";
        ctx.shadowBlur = 0;
    };

    return (
        <div className="holographic-scope">
            <div className="scope-grid"></div>
            <div className="scope-overlay">
                <span>VECTOR SCOPE</span>
                <span>Lissajous: XY</span>
            </div>
            <canvas
                ref={canvasRef}
                width={300}
                height={300}
                className="scope-canvas"
            />
        </div>
    );
};
