import React, { useEffect, useRef, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './StereoImagerCanvas.css';

interface StereoImagerCanvasProps {
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

export const StereoImagerCanvas: React.FC<StereoImagerCanvasProps> = ({ trackId, processorId }) => {
    const [params, setParams] = useState<any[]>([]);
    const gCanvasRef = useRef<HTMLCanvasElement>(null);

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
        if (p.name.toLowerCase().includes("width")) defaultVal = 1.0;
        else if (p.name.toLowerCase().includes("pan") || p.name.toLowerCase().includes("balance")) defaultVal = 0.0;
        handleParamChange(p.id, defaultVal);
    };

    // Goniometer drawing
    useEffect(() => {
        const canvas = gCanvasRef.current;
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        const w = canvas.width;
        const h = canvas.height;
        const centerX = w / 2;
        const centerY = h / 2;

        ctx.clearRect(0, 0, w, h);

        // Background Grid
        ctx.strokeStyle = '#222';
        ctx.beginPath();
        ctx.moveTo(0, centerY); ctx.lineTo(w, centerY);
        ctx.moveTo(centerX, 0); ctx.lineTo(centerX, h);
        ctx.stroke();

        // Rotate for 45 deg view (standard goniometer)
        ctx.save();
        ctx.translate(centerX, centerY);
        ctx.rotate(Math.PI / 4);

        const width = params.find(p => p.name === "Width")?.value || 1.0;

        // Draw Lissajous mockup
        ctx.beginPath();
        ctx.strokeStyle = '#00ffcc';
        ctx.lineWidth = 1;

        for (let i = 0; i < 100; i++) {
            const amp = Math.random() * (h * 0.4);
            const spread = (Math.random() - 0.5) * amp * width;
            const x = spread;
            const y = amp - h * 0.2;

            if (i === 0) ctx.moveTo(x, y);
            else ctx.lineTo(x, y);
        }
        ctx.stroke();
        ctx.restore();

    }, [params]);

    return (
        <div className="stereo-imager-editor">
            <div className="stereo-viz">
                <canvas ref={gCanvasRef} width={250} height={250} />
            </div>
            <div className="imager-controls">
                {params.map(p => (
                    <div key={p.id} className="knob-group">
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
                        <span>{p.value.toFixed(2)}</span>
                    </div>
                ))}
            </div>
        </div>
    );
};
