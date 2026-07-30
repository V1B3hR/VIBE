import React, { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './EqCanvas.css'; // Reuse EQ styling for now

interface FilterCanvasProps {
    trackId: number;
    processorId: string;
    width?: number;
    height?: number;
}

interface Parameter {
    id: string;
    name: string;
    value: number;
    min_value: number;
    max_value: number;
}

export const FilterCanvas: React.FC<FilterCanvasProps> = ({ trackId, processorId, width = 600, height = 300 }) => {
    const [params, setParams] = useState<Parameter[]>([]);
    const canvasRef = useRef<HTMLCanvasElement>(null);

    // Fetch params
    const fetchParams = async () => {
        try {
            const tracks = await invoke<any[]>("get_tracks");
            const track = tracks[trackId];
            if (track) {
                const effect = track.effects.find((fx: any) => fx.id === processorId);
                if (effect) {
                    setParams(effect.parameters);
                }
            }
        } catch (e) {
            console.error("Filter fetch params error", e);
        }
    };

    useEffect(() => {
        fetchParams();
        const interval = setInterval(fetchParams, 100);
        return () => clearInterval(interval);
    }, [trackId, processorId]);

    const getParam = (name: string) => params.find(p => p.name === name);

    const handleParamChange = async (id: string, value: number) => {
        await invoke("set_parameter", { paramId: id, value });
        setParams(prev => prev.map(p => p.id === id ? { ...p, value } : p));
    };

    const handleDoubleClickReset = (p: Parameter) => {
        let defaultVal = (p.min_value + p.max_value) / 2;
        if (p.name === "Cutoff") defaultVal = 1000.0;
        else if (p.name === "Q") defaultVal = 0.707;
        handleParamChange(p.id, defaultVal);
    };

    // Draw Curve
    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        // Visual constants
        const w = width;
        const h = height;
        ctx.clearRect(0, 0, w, h);

        // Draw Grid
        ctx.strokeStyle = '#333';
        ctx.lineWidth = 1;
        ctx.beginPath();
        // Simple log grid lines approximation
        [30, 100, 300, 1000, 3000, 10000].forEach(freq => {
            const x = (Math.log10(freq) - Math.log10(20)) / (Math.log10(20000) - Math.log10(20)) * w;
            ctx.moveTo(x, 0);
            ctx.lineTo(x, h);
        });
        ctx.stroke();

        ctx.strokeStyle = '#333';
        ctx.beginPath();
        // 0dB line
        const y0 = h / 2;
        ctx.moveTo(0, y0);
        ctx.lineTo(w, y0);
        ctx.stroke();

        // Draw Cutoff Line
        const cutoff = getParam("Cutoff")?.value || 1000;
        const cutoffX = (Math.log10(cutoff) - Math.log10(20)) / (Math.log10(20000) - Math.log10(20)) * w;
        ctx.strokeStyle = '#ffff00';
        ctx.setLineDash([5, 5]);
        ctx.beginPath();
        ctx.moveTo(cutoffX, 0);
        ctx.lineTo(cutoffX, h);
        ctx.stroke();
        ctx.setLineDash([]);

        // Calculate Coeffs
        const q = getParam("Q")?.value || 0.707;
        const typeParam = getParam("Type")?.value || 0;
        const type = Math.round(typeParam);

        const sampleRate = 48000;
        const w0 = 2 * Math.PI * cutoff / sampleRate;
        const cosW0 = Math.cos(w0);
        const alpha = Math.sin(w0) / (2 * q);

        let b0 = 0, b1 = 0, b2 = 0, a0 = 0, a1 = 0, a2 = 0;

        if (type === 0) { // LP
            b1 = 1 - cosW0; b0 = b1 / 2; b2 = b0;
            a0 = 1 + alpha; a1 = -2 * cosW0; a2 = 1 - alpha;
        } else if (type === 1) { // HP
            b1 = -(1 + cosW0); b0 = -b1 / 2; b2 = b0;
            a0 = 1 + alpha; a1 = -2 * cosW0; a2 = 1 - alpha;
        } else if (type === 2) { // BP (0dB peak)
            b0 = alpha; b1 = 0; b2 = -alpha;
            a0 = 1 + alpha; a1 = -2 * cosW0; a2 = 1 - alpha;
        } else { // Notch
            b0 = 1; b1 = -2 * cosW0; b2 = 1;
            a0 = 1 + alpha; a1 = -2 * cosW0; a2 = 1 - alpha;
        }

        // Normalize
        b0 /= a0; b1 /= a0; b2 /= a0; a1 /= a0; a2 /= a0;

        ctx.strokeStyle = '#00ffed';
        ctx.lineWidth = 3;
        ctx.shadowColor = '#00ffed';
        ctx.shadowBlur = 10;
        ctx.beginPath();

        for (let x = 0; x < w; x++) {
            // Map x to freq
            const fLog = (x / w) * (Math.log10(20000) - Math.log10(20)) + Math.log10(20);
            const f = Math.pow(10, fLog);

            // Response at freq f
            const w_freq = 2 * Math.PI * f / sampleRate;
            const cosW = Math.cos(w_freq);
            const cos2W = Math.cos(2 * w_freq);

            const num = b0 * b0 + b1 * b1 + b2 * b2 + 2 * (b0 * b1 + b1 * b2) * cosW + 2 * b0 * b2 * cos2W;
            const den = 1 + a1 * a1 + a2 * a2 + 2 * (a1 + a1 * a2) * cosW + 2 * a2 * cos2W;

            const mag = Math.sqrt(num / den);
            const db = 20 * Math.log10(mag);

            // Map dB to Y (+18 to -18)
            const y = y0 - (db / 36) * h;

            if (x === 0) ctx.moveTo(x, y);
            else ctx.lineTo(x, y);
        }
        ctx.stroke();

    }, [params]);

    return (
        <div className="filter-canvas-container" style={{ display: 'flex', flexDirection: 'column', alignItems: 'center' }}>
            <canvas
                ref={canvasRef}
                width={width}
                height={height}
                className="eq-canvas"
                style={{ background: '#111', borderRadius: '4px', border: '1px solid #333' }}
            />
            <div className="filter-controls" style={{ display: 'flex', gap: '20px', marginTop: '15px' }}>
                {/* Type Selector */}
                <div className="control-group">
                    <label>Filter Type</label>
                    <select
                        value={Math.round(getParam("Type")?.value || 0)}
                        onChange={e => {
                            const p = getParam("Type");
                            if (p) handleParamChange(p.id, parseFloat(e.target.value));
                        }}
                        style={{ padding: '5px', background: '#222', color: '#fff', border: '1px solid #444' }}
                    >
                        <option value={0}>Low Pass</option>
                        <option value={1}>High Pass</option>
                        <option value={2}>Band Pass</option>
                        <option value={3}>Notch</option>
                    </select>
                </div>

                {/* Cutoff Slider */}
                <div className="control-group">
                    <label>Cutoff: {(getParam("Cutoff")?.value || 0).toFixed(0)} Hz</label>
                    <input
                        type="range"
                        min={Math.log10(20)}
                        max={Math.log10(20000)}
                        step="0.01"
                        value={Math.log10(getParam("Cutoff")?.value || 1000)}
                        onChange={e => {
                            const val = Math.pow(10, parseFloat(e.target.value));
                            const p = getParam("Cutoff");
                            if (p) handleParamChange(p.id, val);
                        }}
                        onDoubleClick={() => {
                            const p = getParam("Cutoff");
                            if (p) handleDoubleClickReset(p);
                        }}
                    />
                </div>

                {/* Q Slider */}
                <div className="control-group">
                    <label>Resonance (Q): {(getParam("Q")?.value || 0.7).toFixed(2)}</label>
                    <input
                        type="range"
                        min="0.1"
                        max="10"
                        step="0.1"
                        value={getParam("Q")?.value || 0.707}
                        onChange={e => {
                            const p = getParam("Q");
                            if (p) handleParamChange(p.id, parseFloat(e.target.value));
                        }}
                        onDoubleClick={() => {
                            const p = getParam("Q");
                            if (p) handleDoubleClickReset(p);
                        }}
                    />
                </div>
            </div>
        </div>
    );
};
