import React, { useEffect, useRef, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './ConvolutionReverbCanvas.css';

interface ConvolutionReverbCanvasProps {
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

export const ConvolutionReverbCanvas: React.FC<ConvolutionReverbCanvasProps> = ({ trackId, processorId }) => {
    const [params, setParams] = useState<Parameter[]>([]);
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const [activeIR, setActiveIR] = useState("Lexi Hall");

    const fetchParams = useCallback(async () => {
        const tList = await invoke<any[]>("get_tracks");
        const track = tList[trackId];
        if (track) {
            const effect = track.effects.find((fx: any) => fx.id === processorId);
            if (effect) {
                setParams(effect.parameters);
            }
        }
    }, [trackId, processorId]);

    useEffect(() => {
        fetchParams();
    }, [fetchParams]);

    const handleParamChange = async (id: string, value: number) => {
        await invoke("set_parameter", { paramId: id, value });
        setParams(prev => prev.map(p => p.id === id ? { ...p, value } : p));
    };

    // Draw IR Waveform
    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        const w = canvas.width;
        const h = canvas.height;

        ctx.clearRect(0, 0, w, h);

        // Grid
        ctx.strokeStyle = 'rgba(255, 255, 255, 0.05)';
        ctx.beginPath();
        for (let i = 0; i < 10; i++) {
            ctx.moveTo(i * (w / 10), 0);
            ctx.lineTo(i * (w / 10), h);
        }
        ctx.stroke();

        // IR Particle Waveform logic
        // We'll generate a "ghost waveform" inspired by Lexicon style
        const points = 300;
        const step = w / points;

        const decay = params.find(p => p.name === "Size")?.value || 1.0;

        ctx.beginPath();
        ctx.moveTo(0, h / 2);

        for (let i = 0; i < points; i++) {
            const progress = i / points;
            const x = i * step;
            // Exponential decay envelope
            const amp = Math.exp(-progress * 5 / decay) * (h / 2.5);
            // Random jitter for "room reflections"
            const noise = (Math.random() - 0.5) * amp * (0.2 + progress * 0.8);
            const y = h / 2 + noise;

            ctx.lineTo(x, y);
        }

        const grad = ctx.createLinearGradient(0, 0, w, 0);
        grad.addColorStop(0, '#00ffcc');
        grad.addColorStop(0.5, '#00ffff');
        grad.addColorStop(1, 'rgba(0, 255, 255, 0)');

        ctx.strokeStyle = grad;
        ctx.lineWidth = 1.5;
        ctx.shadowBlur = 15;
        ctx.shadowColor = '#00ffcc';
        ctx.stroke();
        ctx.shadowBlur = 0;

        // Fill under
        ctx.lineTo(w, h / 2);
        ctx.lineTo(0, h / 2);
        ctx.fillStyle = 'rgba(0, 255, 204, 0.05)';
        ctx.fill();

    }, [params]);

    return (
        <div className="reverb-editor-vibe">
            <div className="preset-strip">
                {["Lexi Hall", "Studio B", "Church", "Plate 140", "Ambient Spark"].map(name => (
                    <button
                        key={name}
                        className={`btn-ir-preset ${activeIR === name ? 'active' : ''}`}
                        onClick={() => setActiveIR(name)}
                    >
                        {name}
                    </button>
                ))}
            </div>

            <div className="reverb-viz-container">
                <canvas ref={canvasRef} width={600} height={200} className="reverb-canvas" />
                <div style={{ position: 'absolute', top: 10, right: 10, fontSize: '0.6rem', color: '#444' }}>
                    REAL-TIME UPC CONVOLUTION ENGINE
                </div>
            </div>

            <div className="reverb-controls">
                {params.map(p => (
                    <div key={p.id} className="knob-unit">
                        <label>{p.name}</label>
                        <input
                            type="range"
                            min={p.min_value}
                            max={p.max_value}
                            step="0.01"
                            value={p.value}
                            onChange={(e) => handleParamChange(p.id, parseFloat(e.target.value))}
                        />
                        <span className="val-display">
                            {p.name === "Mix" ? (p.value * 100).toFixed(0) + "%" : p.value.toFixed(2)}
                        </span>
                    </div>
                ))}
            </div>
        </div>
    );
};
