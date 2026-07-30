import React, { useEffect, useRef, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './SpectralGateCanvas.css';

interface SpectralGateCanvasProps {
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

export const SpectralGateCanvas: React.FC<SpectralGateCanvasProps> = ({ trackId, processorId }) => {
    const [params, setParams] = useState<any[]>([]);
    const canvasRef = useRef<HTMLCanvasElement>(null);

    const fetchParams = useCallback(async () => {
        const tList = await invoke<any[]>("get_tracks");
        const track = tList[trackId];
        if (track) {
            const effect = track.effects.find((fx: any) => fx.id === processorId);
            if (effect) setParams(effect.parameters);
        }
    }, [trackId, processorId]);

    useEffect(() => { fetchParams(); }, [fetchParams]);

    const handleParamChange = async (id: string, value: number) => {
        await invoke("set_parameter", { paramId: id, value });
        setParams(prev => prev.map(p => p.id === id ? { ...p, value } : p));
    };

    const handleDoubleClickReset = (p: Parameter) => {
        let defaultVal = (p.min_value + p.max_value) / 2;
        if (p.name === "Threshold") defaultVal = -30.0;
        else if (p.name === "Ratio") defaultVal = 4.0;
        else if (p.name === "Attack") defaultVal = 10.0;
        else if (p.name === "Release") defaultVal = 100.0;
        handleParamChange(p.id, defaultVal);
    };

    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        const w = canvas.width;
        const h = canvas.height;

        ctx.clearRect(0, 0, w, h);

        // Simple frequency bin logic (visual simulation)
        const threshold = params.find(p => p.name.includes("Thr"))?.value || -40;

        ctx.fillStyle = '#111';
        ctx.fillRect(0, 0, w, h);

        // Threshold line
        const threshY = ((threshold + 100) / 100) * h;
        ctx.strokeStyle = 'rgba(255, 0, 0, 0.5)';
        ctx.setLineDash([5, 5]);
        ctx.beginPath();
        ctx.moveTo(0, h - threshY);
        ctx.lineTo(w, h - threshY);
        ctx.stroke();
        ctx.setLineDash([]);

        // Mock spectral bins
        for (let i = 0; i < 60; i++) {
            const binH = Math.random() * h * 0.8;
            const x = i * (w / 60);
            const passed = binH > (h - threshY);

            ctx.fillStyle = passed ? '#00ccff' : '#333';
            ctx.fillRect(x + 1, h - binH, (w / 60) - 2, binH);
        }

    }, [params]);

    return (
        <div className="spectral-gate-editor">
            <div className="spectral-viz">
                <canvas ref={canvasRef} width={500} height={150} />
            </div>
            <div className="spectral-controls">
                {params.map(p => (
                    <div key={p.id} className="knob-group">
                        <label>{p.name}</label>
                        <input
                            type="range"
                            min={p.min_value}
                            max={p.max_value}
                            step="0.1"
                            value={p.value ?? 0}
                            onChange={(e) => handleParamChange(p.id, parseFloat(e.target.value))}
                            onDoubleClick={() => handleDoubleClickReset(p)}
                        />
                        <span>{(p.value ?? 0).toFixed(1)}</span>
                    </div>
                ))}
            </div>
        </div>
    );
};
