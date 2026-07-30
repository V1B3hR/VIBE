import React, { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './TubeLimiterCanvas.css';

interface TubeLimiterCanvasProps {
    trackId: number;
    processorId: string;
}

interface Parameter {
    id: string;
    name: string;
    value: number;
    min_value: number;
    max_value: number;
}

export const TubeLimiterCanvas: React.FC<TubeLimiterCanvasProps> = ({ trackId, processorId }) => {
    const [params, setParams] = useState<Parameter[]>([]);
    const canvasRef = useRef<HTMLCanvasElement>(null);

    const fetchParams = async () => {
        const tracks = await invoke<any[]>("get_tracks");
        const track = tracks[trackId];
        if (track) {
            const effect = track.effects.find((fx: any) => fx.id === processorId);
            if (effect) {
                setParams(effect.parameters);
            }
        }
    };

    useEffect(() => {
        fetchParams();
        const interval = setInterval(fetchParams, 100);
        return () => clearInterval(interval);
    }, [trackId, processorId]);

    const findParam = (name: string) => params.find(p => p.name === name);

    const handleParamChange = async (id: string, value: number) => {
        await invoke("set_parameter", { paramId: id, value });
        setParams(prev => prev.map(p => p.id === id ? { ...p, value } : p));
    };

    const handleDoubleClickReset = (p: Parameter) => {
        let defaultVal = (p.min_value + p.max_value) / 2;
        const nameLower = p.name.toLowerCase();
        if (nameLower.includes("drive") || nameLower.includes("gain") || nameLower.includes("ceiling") || nameLower.includes("limit")) {
            defaultVal = 0.0;
        } else if (nameLower.includes("release")) {
            defaultVal = 100.0;
        } else if (nameLower.includes("warmth")) {
            defaultVal = 0.5;
        }
        handleParamChange(p.id, defaultVal);
    };

    // Draw Saturation Curve
    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        const w = canvas.width;
        const h = canvas.height;
        ctx.clearRect(0, 0, w, h);

        const drive = findParam("Tube Drive")?.value || 0.5;
        const inputGain = findParam("Input Gain")?.value || 0;
        const ceiling = findParam("Ceiling")?.value || -0.1;

        const ceilingLin = Math.pow(10, ceiling / 20);
        const gainLin = Math.pow(10, inputGain / 20);

        // Grid
        ctx.strokeStyle = '#222';
        ctx.lineWidth = 1;
        ctx.beginPath();
        for (let i = 0; i <= 4; i++) {
            const x = (i / 4) * w;
            const y = (i / 4) * h;
            ctx.moveTo(x, 0); ctx.lineTo(x, h);
            ctx.moveTo(0, y); ctx.lineTo(w, y);
        }
        ctx.stroke();

        // Saturation Curve
        ctx.strokeStyle = '#d4af37';
        ctx.lineWidth = 3;
        ctx.shadowBlur = 10;
        ctx.shadowColor = 'rgba(212, 175, 55, 0.5)';
        ctx.beginPath();

        const softClip = (x: number, d: number) => {
            const scaled = x * (1.0 + d * 2.0);
            if (scaled > 0) {
                return (2.0 / Math.PI) * Math.atan(scaled * Math.PI / 2.0);
            } else {
                return 0.9 * (2.0 / Math.PI) * Math.atan(scaled * 1.1 * Math.PI / 2.0);
            }
        };

        for (let i = 0; i < w; i++) {
            const xNorm = (i / w) * 2 - 1; // -1 to 1
            let yNorm = xNorm * gainLin;

            // Apply saturation
            yNorm = softClip(yNorm, drive);

            // Apply ceiling (clamp)
            yNorm = Math.max(-ceilingLin, Math.min(ceilingLin, yNorm));

            const canvasX = i;
            const canvasY = h / 2 - (yNorm * h / 2);

            if (i === 0) ctx.moveTo(canvasX, canvasY);
            else ctx.lineTo(canvasX, canvasY);
        }
        ctx.stroke();
        ctx.shadowBlur = 0;

        // Origin lines
        ctx.strokeStyle = '#333';
        ctx.setLineDash([2, 4]);
        ctx.beginPath();
        ctx.moveTo(w / 2, 0); ctx.lineTo(w / 2, h);
        ctx.moveTo(0, h / 2); ctx.lineTo(w, h / 2);
        ctx.stroke();
        ctx.setLineDash([]);

    }, [params]);

    return (
        <div className="tube-container">
            <div className="tube-header">
                <h3>MAGNETO-TUBE v1.0</h3>
                <span className="tube-badge">VARIABLE-MU</span>
            </div>
            <div className="tube-viz">
                <div className="transfer-function">
                    <label>TRANSFER CHARACTERISTIC</label>
                    <canvas ref={canvasRef} width={400} height={200} />
                </div>
            </div>
            <div className="tube-controls">
                {params.map(p => {
                    if (p.name === "True Peak") {
                        return (
                            <div key={p.id} className="tube-param toggle-row">
                                <label>{p.name}</label>
                                <button
                                    className={`tube-toggle ${p.value > 0.5 ? 'active' : ''}`}
                                    onClick={() => handleParamChange(p.id, p.value > 0.5 ? 0.0 : 1.0)}
                                >
                                    {p.value > 0.5 ? 'ENABLED' : 'DISABLED'}
                                </button>
                            </div>
                        );
                    }
                    return (
                        <div key={p.id} className="tube-param">
                            <label>{p.name}</label>
                            <input
                                type="range"
                                min={p.min_value}
                                max={p.max_value}
                                step="0.01"
                                value={p.value}
                                onChange={(e) => handleParamChange(p.id, parseFloat(e.target.value))}
                                onDoubleClick={() => handleDoubleClickReset(p)}
                            />
                            <span>{p.value.toFixed(2)} {p.name.includes("Gain") || p.name === "Ceiling" ? "dB" : p.name === "Release" ? "ms" : ""}</span>
                        </div>
                    );
                })}
            </div>
        </div>
    );
};
