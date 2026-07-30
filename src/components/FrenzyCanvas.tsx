import React, { useEffect, useState, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './FrenzyCanvas.css';

interface FrenzyCanvasProps {
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

export const FrenzyCanvas: React.FC<FrenzyCanvasProps> = ({ trackId, processorId }) => {
    const [params, setParams] = useState<Parameter[]>([]);
    const containerRef = useRef<HTMLDivElement>(null);

    const fetchParams = async () => {
        try {
            const tracks = await invoke<any[]>("get_tracks");
            // Find track by ID (some implementations might use index, some UUID string)
            const track = tracks.find((t: any, idx: number) => idx === trackId || t.id === trackId);
            if (track) {
                const effect = track.effects.find((fx: any) => fx.id === processorId);
                if (effect) {
                    setParams(effect.parameters);
                }
            }
        } catch (e) {
            console.error("Frenzy fetch params error", e);
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
        if (p.name === "Frenzy Count") defaultVal = 1;
        else if (p.name === "Ice") defaultVal = 0.0;
        else if (p.name === "Feedback") defaultVal = 0.5;
        else if (p.name === "Tone") defaultVal = 0.5;
        handleParamChange(p.id, defaultVal);
    };

    const renderControl = (paramName: string, label: string, step = 0.01, unit = "") => {
        const p = getParam(paramName);
        if (!p) return null;
        return (
            <div className="frenzy-control-group">
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    <label className="frenzy-label">{label}</label>
                    <span className="frenzy-value">
                        {p.value.toFixed(paramName === "Frenzy Count" ? 0 : 1)}{unit}
                    </span>
                </div>
                <input
                    type="range"
                    className="frenzy-slider"
                    min={p.min_value}
                    max={p.max_value}
                    step={step}
                    value={p.value}
                    onChange={e => handleParamChange(p.id, parseFloat(e.target.value))}
                    onDoubleClick={() => handleDoubleClickReset(p)}
                />
            </div>
        );
    };

    const frenzyCount = Math.round(getParam("Frenzy Count")?.value || 1);
    const iceValue = getParam("Ice")?.value || 0;

    return (
        <div className="frenzy-container" ref={containerRef}>
            <div className="frenzy-header">
                <h3 className="frenzy-title">Frenzy Multiplier</h3>
                <div style={{ fontSize: '9px', color: '#00ffff66', letterSpacing: '1px' }}>REAPER EDITION</div>
            </div>

            <div className="frenzy-viz">
                {/* Dynamic Chaos Visualization */}
                {[...Array(frenzyCount)].map((_, i) => (
                    <div
                        key={i}
                        className="frenzy-particle"
                        style={{
                            left: `${20 + (i * (60 / frenzyCount)) + Math.random() * 10}%`,
                            animationDelay: `${i * 0.2}s`,
                            background: iceValue > 0.5 ? '#ff00ff' : '#00ffff',
                            boxShadow: `0 0 10px ${iceValue > 0.5 ? '#ff00ff' : '#00ffff'}`,
                            filter: `hue-rotate(${i * 20}deg)`
                        }}
                    />
                ))}

                {/* Background Grid */}
                <div style={{
                    position: 'absolute', width: '100%', height: '100%',
                    background: 'linear-gradient(rgba(0,255,255,0.05) 1px, transparent 1px), linear-gradient(90deg, rgba(0,255,255,0.05) 1px, transparent 1px)',
                    backgroundSize: '10px 10px',
                    opacity: 0.3
                }} />

                <div style={{
                    position: 'absolute', bottom: '10px', width: '80%', height: '1px',
                    background: 'linear-gradient(90deg, transparent, #00ffff, transparent)',
                    boxShadow: '0 0 15px #00ffff',
                    opacity: 0.5
                }} />
            </div>

            <div className="frenzy-grid">
                {renderControl("Frenzy Count", "Frenzy", 1)}
                {renderControl("Scatter", "Scatter", 1, "ms")}
                {renderControl("Pitch Chaos", "Pitch Chaos", 0.01)}
                {renderControl("Warmth", "Warmth", 0.01)}
                {renderControl("Ice", "Ice", 0.01)}
                {renderControl("Space", "Space", 0.01)}
            </div>

            <div style={{
                marginTop: '10px',
                fontSize: '9px',
                color: '#444',
                textAlign: 'center',
                textTransform: 'uppercase',
                letterSpacing: '2px'
            }}>
                Gemini Multi-Tap DSP // v1.0
            </div>

            {iceValue > 0.8 && (
                <div style={{
                    position: 'absolute', top: '10px', left: '0', width: '100%',
                    textAlign: 'center', pointerEvents: 'none',
                    color: '#ff00ff', fontSize: '10px', fontWeight: 'bold',
                    textShadow: '0 0 5px #ff00ff'
                }}>
                    CRITICAL FREEZE
                </div>
            )}
        </div>
    );
};
