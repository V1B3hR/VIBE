import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './EqCanvas.css'; // Reuse basic styling

interface SaturationCanvasProps {
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

export const SaturationCanvas: React.FC<SaturationCanvasProps> = ({ trackId, processorId }) => {
    const [params, setParams] = useState<Parameter[]>([]);

    // UI State for tabs/types
    // const [currentType, setCurrentType] = useState('Tube');

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
            console.error("Saturation fetch params error", e);
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
        const nameLower = p.name.toLowerCase();
        if (nameLower.includes("drive")) defaultVal = 2.0;
        else if (nameLower.includes("mix")) defaultVal = 0.5;
        else if (nameLower.includes("ceiling") || nameLower.includes("limit")) defaultVal = 0.0;
        handleParamChange(p.id, defaultVal);
    };

    /*
    const renderKnob = (paramName: string, label: string, step = 0.01, unit = "") => {
        const p = getParam(paramName);
        if (!p) return null;
        return (
            <div className="control-group" style={{ display: 'flex', flexDirection: 'column', gap: '4px', alignItems: 'center' }}>
                <label style={{ fontSize: '10px', color: '#888', textTransform: 'uppercase' }}>{label}</label>
                <div style={{ position: 'relative', width: '60px', height: '60px', display: 'flex', justifyContent: 'center', alignItems: 'center' }}>
                    <input
                        type="range"
                        min={p.min_value}
                        max={p.max_value}
                        step={step}
                        value={p.value}
                        onChange={e => handleParamChange(p.id, parseFloat(e.target.value))}
                        className="knob-range" // Would need rotation CSS
                        style={{ width: '100%', cursor: 'pointer' }}
                    />
                </div>
                <span style={{ fontSize: '12px', color: '#ffaa00', fontFamily: 'monospace', marginTop: '-10px' }}>
                    {p.value.toFixed(1)}{unit}
                </span>
            </div>
        );
    };
    */

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
                        style={{ flex: 1 }}
                    />
                    <span style={{ fontSize: '11px', color: '#ffaa00', width: '35px', textAlign: 'right' }}>
                        {(p.value ?? 0).toFixed(1)}{unit}
                    </span>
                </div>
            </div>
        );
    };

    const renderTypeSelector = () => {
        const p = getParam("Type");
        if (!p) return null;
        const current = Math.round(p.value);

        const types = ["TUBE", "TAPE", "SOLID"];

        return (
            <div style={{ display: 'flex', gap: '5px', marginBottom: '15px', justifyContent: 'center' }}>
                {types.map((t, idx) => (
                    <button
                        key={t}
                        onClick={() => handleParamChange(p.id, idx)}
                        style={{
                            background: current === idx ? '#ffaa00' : '#222',
                            color: current === idx ? '#000' : '#666',
                            border: '1px solid #444',
                            padding: '4px 10px',
                            borderRadius: '4px',
                            fontSize: '10px',
                            fontWeight: 'bold',
                            cursor: 'pointer',
                            flex: 1
                        }}
                    >
                        {t}
                    </button>
                ))}
            </div>
        )
    };

    const destroyMode = (getParam("Destroy")?.value || 0) > 0.5;

    return (
        <div className="vibe-saturation-color" style={{
            display: 'flex', flexDirection: 'column', padding: '20px',
            background: 'linear-gradient(145deg, #1a1a1a 0%, #0d0d0d 100%)', color: '#eee',
            borderRadius: '12px', width: '300px',
            border: '1px solid rgba(255,170,0,0.1)',
            boxShadow: '0 20px 40px rgba(0,0,0,0.6)'
        }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '15px' }}>
                <h3 style={{ margin: 0, fontSize: '14px', letterSpacing: '2px', color: '#aaa', textTransform: 'uppercase' }}>
                    COLOR BOX
                </h3>
                {renderTypeSelector()}
            </div>

            <div className="saturation-visualizer" style={{
                height: '80px', background: '#050505',
                borderRadius: '6px', marginBottom: '20px',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                border: '1px solid #222',
                boxShadow: destroyMode ? '0 0 20px rgba(255,0,0,0.3) inset' : 'none'
            }} >
                {/* Simple Curve viz placeholder */}
                <div style={{
                    width: '100%', height: '2px', background: '#333', position: 'relative'
                }}>
                    <div style={{
                        position: 'absolute', top: '50%', left: '0', width: '100%', height: '2px',
                        background: destroyMode ? 'red' : '#ffaa00',
                        transform: `scaleY(${getParam("Drive")?.value ? (getParam("Drive")!.value / 10 + 1) : 1})`,
                        opacity: 0.8
                    }} />
                </div>
                {destroyMode && <div style={{ color: 'red', fontWeight: 'bold', fontSize: '12px', position: 'absolute' }}>DESTROY ACTIVE</div>}
            </div>

            <div className="sat-controls" style={{ display: 'flex', flexDirection: 'column', gap: '15px' }}>
                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '20px' }}>
                    {renderSlider("Drive", "Drive", 0.1, "dB")}
                    {renderSlider("Output", "Output", 0.1, "dB")}
                </div>

                {renderSlider("Bias", "DC Bias / Asym")}
                {renderSlider("Focus", "Focus (Low Clean)", 1.0, "Hz")}
                {renderSlider("Mix", "Dry / Wet Mix", 0.01)}

                <div className="destroy-switch" style={{ marginTop: '10px' }}>
                    {(() => {
                        const p = getParam("Destroy");
                        if (!p) return null;
                        const isOn = p.value > 0.5;
                        return (
                            <button
                                onClick={() => handleParamChange(p.id, isOn ? 0.0 : 1.0)}
                                style={{
                                    width: '100%',
                                    background: isOn ? '#ff0000' : '#222',
                                    color: isOn ? '#fff' : '#888',
                                    border: '1px solid #444',
                                    padding: '8px',
                                    borderRadius: '4px',
                                    fontWeight: 'bold',
                                    cursor: 'pointer',
                                    letterSpacing: '2px',
                                    fontSize: '11px'
                                }}
                            >
                                ☢ DESTROY MODE
                            </button>
                        )
                    })()}
                </div>
            </div>
        </div>
    );
};
