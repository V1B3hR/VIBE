import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './EqCanvas.css';

interface DelayCanvasProps {
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

export const DelayCanvas: React.FC<DelayCanvasProps> = ({ trackId, processorId }) => {
    const [params, setParams] = useState<Parameter[]>([]);

    // Viz state
    // const [vizHeads, setVizHeads] = useState<number[]>([]);

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
            console.error("Delay fetch params error", e);
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
        if (p.name === "Feedback") defaultVal = 0.3;
        else if (p.name === "Mix" || p.name === "Wet Mix") defaultVal = 0.5;
        else if (p.name.includes("Time")) defaultVal = 250.0;
        else if (p.name === "Diffusion") defaultVal = 0.0;
        else if (p.name === "Tape Warble") defaultVal = 0.0;
        handleParamChange(p.id, defaultVal);
    };

    // Derived State
    const isSync = (getParam("Sync")?.value || 0) > 0.5;
    const isPingPong = (getParam("PingPong")?.value || 0) > 0.5;

    // Texture State
    const diffusionVal = getParam("Diffusion")?.value || 0;
    const warbleVal = getParam("Tape Warble")?.value || 0;

    const renderSlider = (paramName: string, label: string, step = 0.01, unit = "") => {
        const p = getParam(paramName);
        if (!p) return null;
        return (
            <div className="control-group" style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                <label style={{ fontSize: '10px', color: '#888', textTransform: 'uppercase' }}>{label}</label>
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <input
                        type="range"
                        className="vibe-slider"
                        min={p.min_value}
                        max={p.max_value}
                        step={step}
                        value={p.value}
                        onChange={e => handleParamChange(p.id, parseFloat(e.target.value))}
                        onDoubleClick={() => handleDoubleClickReset(p)}
                        disabled={isSync && (paramName.startsWith("Time"))}
                        style={{ flex: 1, opacity: (isSync && paramName.startsWith("Time")) ? 0.3 : 1 }}
                    />
                    <span style={{ fontSize: '11px', color: '#00ffed', width: '35px', textAlign: 'right' }}>
                        {p.value.toFixed(step >= 1 ? 0 : 2)}{unit}
                    </span>
                </div>
            </div>
        );
    };

    const renderSwitch = (paramName: string, label: string) => {
        const p = getParam(paramName);
        if (!p) return null;
        const isOn = p.value > 0.5;
        return (
            <div className="control-group" style={{ flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between' }}>
                <label style={{ fontSize: '10px', color: '#aaa', textTransform: 'uppercase' }}>{label}</label>
                <button
                    onClick={() => handleParamChange(p.id, isOn ? 0.0 : 1.0)}
                    style={{
                        background: isOn ? '#00ffed' : '#222',
                        color: isOn ? '#000' : '#888',
                        border: '1px solid #444',
                        borderRadius: '4px',
                        padding: '2px 8px',
                        fontSize: '9px',
                        cursor: 'pointer',
                        fontWeight: 'bold',
                        transition: 'all 0.2s',
                        boxShadow: isOn ? '0 0 10px rgba(0,255,237,0.3)' : 'none'
                    }}
                >
                    {isOn ? "ON" : "OFF"}
                </button>
            </div>
        );
    };

    return (
        <div className="vibe-delay-texture" style={{
            display: 'flex', flexDirection: 'column', padding: '20px',
            background: '#0a0b0d', color: '#eee',
            borderRadius: '12px', width: '400px',
            border: '1px solid rgba(255,255,255,0.05)',
            boxShadow: '0 20px 40px rgba(0,0,0,0.6)'
        }}>
            <h3 style={{ margin: '0 0 15px 0', fontSize: '14px', letterSpacing: '2px', color: '#555', textTransform: 'uppercase' }}>
                ECHO TEXTURE
            </h3>

            <div className="delay-visualizer" style={{
                height: '100px', background: '#000',
                borderRadius: '6px', marginBottom: '20px',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                border: '1px solid #1a1a1a', position: 'relative', overflow: 'hidden'
            }} >
                {/* Visual Representation of Delay Taps */}
                <div style={{
                    display: 'flex', gap: isPingPong ? '40px' : '20px',
                    opacity: 0.8, filter: `blur(${diffusionVal * 4}px)`
                }}>
                    {[1, 2, 3, 4].map(i => (
                        <div key={i} style={{
                            width: '4px',
                            height: `${40 / i}px`,
                            background: '#00ffed',
                            borderRadius: '2px',
                            transform: `translateY(${Math.sin(Date.now() / 500 * warbleVal * i) * 10}px)`
                        }} />
                    ))}
                </div>

                {isPingPong && <div style={{ position: 'absolute', bottom: 5, right: 5, fontSize: '9px', color: '#00ffed' }}>PING PONG</div>}
                {(getParam("Ducker")?.value || 0) > 0.1 && <div style={{ position: 'absolute', bottom: 5, left: 5, fontSize: '9px', color: '#ff9900' }}>DUCKING</div>}
            </div>

            <div className="delay-controls-grid" style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '20px' }}>

                {/* Timing Column */}
                <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
                    <h4 style={{ margin: 0, fontSize: '10px', color: '#555' }}>TIMING</h4>
                    {renderSwitch("Sync", "Tempo Sync")}
                    {renderSwitch("PingPong", "Ping Pong")}

                    {isSync ? (
                        <div className="control-group" style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                            <label style={{ fontSize: '10px', color: '#888' }}>NOTE</label>
                            <select
                                value={Math.round(getParam("Sync Note")?.value || 2)}
                                onChange={e => {
                                    const p = getParam("Sync Note");
                                    if (p) handleParamChange(p.id, parseFloat(e.target.value));
                                }}
                                style={{ padding: '4px', background: '#222', color: '#00ffed', border: '1px solid #444', borderRadius: '4px', fontSize: '11px' }}
                            >
                                <option value={0}>1/1 Whole</option>
                                <option value={1}>1/2 Half</option>
                                <option value={2}>1/4 Quarter</option>
                                <option value={3}>1/8 Eighth</option>
                                <option value={4}>1/16 Sixteenth</option>
                            </select>
                        </div>
                    ) : (
                        <>
                            {renderSlider("Time L", "Time L", 0.01, "s")}
                            {renderSlider("Time R", "Time R", 0.01, "s")}
                        </>
                    )}
                    {renderSlider("Feedback", "Repeats")}
                </div>

                {/* Texture Column */}
                <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
                    <h4 style={{ margin: 0, fontSize: '10px', color: '#555' }}>TEXTURE</h4>
                    {renderSlider("Diffusion", "Blur / Diffuse")}
                    {renderSlider("Tape Warble", "Tape Flutter")}
                    {renderSlider("Ducker", "Sidechain Duck")}
                    {renderSlider("LP Color", "High Cut", 100, "Hz")}
                </div>
            </div>

            <div className="mix-section" style={{ marginTop: '20px', borderTop: '1px solid #222', paddingTop: '15px' }}>
                {renderSlider("Mix", "Wet / Dry Mix", 0.01, "%")}
            </div>
        </div>
    );
};
