import React, { useEffect, useState, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './NatureFX.css';

interface ReverbCanvasProps {
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

export const ReverbCanvas: React.FC<ReverbCanvasProps> = ({ trackId, processorId }) => {
    const [params, setParams] = useState<Parameter[]>([]);
    const canvasRef = useRef<HTMLCanvasElement>(null);

    const fetchParams = async () => {
        try {
            const tracks = await invoke<any[]>("get_tracks");
            const track = tracks[trackId];
            if (track) {
                let effect = track.effects.find((fx: any) => fx.id === processorId);
                if (effect) {
                    setParams(effect.parameters);
                }
            }
        } catch (e) {
            console.error("Reverb fetch params error", e);
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
        if (p.name === "Size" || p.name === "Room Size") defaultVal = 0.7;
        else if (p.name === "Pre-Delay") defaultVal = 10.0;
        else if (p.name === "Mix" || p.name === "Wet" || p.name === "Wet Mix") defaultVal = 0.3;
        else if (p.name === "Width") defaultVal = 1.0;
        else if (p.name === "Damping") defaultVal = 0.5;
        handleParamChange(p.id, defaultVal);
    };

    // Organic Particle Canvas Animation
    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        let animationId: number;
        let time = 0;

        // "Bioluminescent Forest Spores" algorithm
        const drawOrganicSpace = () => {
            time += 0.02;
            const w = canvas.width;
            const h = canvas.height;

            const sizeVal = getParam("Size")?.value || getParam("Room Size")?.value || 1.0;
            const mixVal = getParam("Mix")?.value || getParam("Wet")?.value || 0.5;

            // Clear with dark, organic, deep-water green/blue
            ctx.fillStyle = 'rgba(7, 12, 10, 0.4)'; // trails effect
            ctx.fillRect(0, 0, w, h);

            // Center glow (The Space Source)
            const cx = w / 2;
            const cy = h / 2;
            
            const radius = 20 + (sizeVal * 60) + Math.sin(time * 2) * 5;
            
            const centerGrad = ctx.createRadialGradient(cx, cy, 0, cx, cy, radius * 1.5);
            centerGrad.addColorStop(0, `rgba(50, 255, 150, ${0.4 * mixVal})`);
            centerGrad.addColorStop(0.5, `rgba(0, 150, 200, ${0.1 * mixVal})`);
            centerGrad.addColorStop(1, 'rgba(0, 0, 0, 0)');
            
            ctx.fillStyle = centerGrad;
            ctx.fillRect(0, 0, w, h);

            // Draw organic rings/ripples
            ctx.lineWidth = 1;
            for (let i = 1; i <= 3; i++) {
                const ringRad = (time * 15 * i) % (radius * 2);
                const alpha = Math.max(0, 1 - (ringRad / (radius * 2)));
                ctx.strokeStyle = `rgba(100, 255, 200, ${alpha * mixVal * 0.5})`;
                ctx.beginPath();
                // Sine wave distortion on circle
                for(let a=0; a<Math.PI*2; a+=0.1) {
                    const distortion = Math.sin(a * 4 + time + i) * (5 * sizeVal);
                    const rx = cx + Math.cos(a) * (ringRad + distortion);
                    const ry = cy + Math.sin(a) * (ringRad + distortion);
                    if (a===0) ctx.moveTo(rx, ry);
                    else ctx.lineTo(rx, ry);
                }
                ctx.closePath();
                ctx.stroke();
            }

            // Draw floating "spores"
            const numSpores = Math.floor(sizeVal * 30);
            ctx.fillStyle = 'rgba(200, 255, 220, 0.8)';
            for(let i = 0; i < numSpores; i++) {
                const angle = (i * 1.34) + time * (i % 2 === 0 ? 0.2 : -0.2);
                const dist = 5 + (i * 2 * sizeVal) + Math.sin(time * 3 + i) * 10;
                ctx.beginPath();
                ctx.arc(cx + Math.cos(angle) * dist, cy + Math.sin(angle) * dist, 1 + (i%2), 0, Math.PI*2);
                ctx.fill();
            }

            animationId = requestAnimationFrame(drawOrganicSpace);
        };

        drawOrganicSpace();

        return () => cancelAnimationFrame(animationId);
    }, [params]);

    const renderOrganicKnob = (paramName: string, label: string) => {
        const p = getParam(paramName);
        if (!p) return null;
        
        // Normalize 0..1 for UI rotation
        const norm = (p.value - p.min_value) / (p.max_value - p.min_value);
        const degrees = -140 + (norm * 280);

        return (
            <div className="organic-knob-container">
                <canvas 
                    className="knob-glow-ring" 
                    width={50} height={50}
                    style={{
                        position: 'absolute',
                        opacity: 0.5 + (norm * 0.5),
                        boxShadow: `0 0 ${10 + norm * 15}px rgba(50, 255, 150, ${norm * 0.4})`,
                        borderRadius: '50%'
                    }}
                />
                <div className="organic-knob-base">
                    <div className="organic-knob-indicator" style={{ transform: `rotate(${degrees}deg)` }}>
                        <div className="indicator-dot"></div>
                    </div>
                </div>
                {/* Invisible range overlay to hijack interactions simply */}
                <input
                    type="range"
                    min={p.min_value}
                    max={p.max_value}
                    step={(p.max_value - p.min_value) / 100}
                    value={p.value}
                    onChange={e => handleParamChange(p.id, parseFloat(e.target.value))}
                    onDoubleClick={() => handleDoubleClickReset(p)}
                    className="invisible-slider"
                />
                <div className="organic-knob-label">
                    <span>{label}</span>
                    <span className="val">{p.value.toFixed(2)}</span>
                </div>
            </div>
        );
    };

    return (
        <div className="vibe-plugin-wrapper nature-skin">
            <div className="plugin-header">
                <h3>VIBE CONVOLUTION</h3>
                <span className="nature-badge">🌿 ORGANIC ENGINE</span>
            </div>

            <div className="visualizer-stage">
                <canvas ref={canvasRef} width={360} height={160} className="organic-canvas" />
                <div className="impulse-hud">
                    IMPULSE: <strong>LUSH FOREST.wav</strong>
                    <br/>
                    <span style={{color: '#888', fontSize: '9px'}}>FFT Partitioned (Zero Latency)</span>
                </div>
            </div>

            <div className="organic-controls">
                <div className="control-tier">
                    {renderOrganicKnob("Size", "Space Size") || renderOrganicKnob("Room Size", "Size")}
                    {renderOrganicKnob("Pre-Delay", "Pre-Delay")}
                    {renderOrganicKnob("Mix", "Dry/Wet") || renderOrganicKnob("Wet", "Wet Mix")}
                </div>
                {/* Render fallback for other params if they exist */}
                <div className="control-tier secondary">
                    {params.filter(p => !["Size", "Room Size", "Pre-Delay", "Mix", "Wet"].includes(p.name)).map(p => (
                        <div key={p.id} className="simple-slider-row">
                            <label>{p.name}</label>
                            <input
                                type="range"
                                min={p.min_value}
                                max={p.max_value}
                                step="0.01"
                                value={p.value}
                                onChange={e => handleParamChange(p.id, parseFloat(e.target.value))}
                            />
                            <span>{p.value.toFixed(2)}</span>
                        </div>
                    ))}
                </div>
            </div>
        </div>
    );
};
