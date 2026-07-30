import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './SynthCanvas.css';
import { AdsrGraph } from './AdsrGraph';

import { HolographicScope } from './HolographicScope';
import { SynthXYPad } from './SynthXYPad';
import ModMatrix, { ModSlot } from './ModMatrix';
import PresetBrowser from './PresetBrowser';

interface SynthCanvasProps {
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

export const SynthCanvas: React.FC<SynthCanvasProps> = ({ trackId, processorId }) => {
    const [params, setParams] = useState<Parameter[]>([]);
    const [activeTab, setActiveTab] = useState<'osc' | 'filter' | 'global' | 'seq' | 'mod'>('osc');
    const [modMatrix, setModMatrix] = useState<ModSlot[]>([]);
    const [procIdx, setProcIdx] = useState<number>(-1);

    const fetchParams = async () => {
        try {
            const tracks = await invoke<any[]>("get_tracks");
            const track = tracks[trackId];
            if (track) {
                const pIndex = track.effects.findIndex((fx: any) => fx.id === processorId);
                if (pIndex !== -1) {
                    setProcIdx(pIndex);
                    const effect = track.effects[pIndex];
                    setParams(effect.parameters);
                    if (effect.mod_matrix) {
                        setModMatrix(effect.mod_matrix);
                    }
                }
            }
        } catch (e) {
            console.error("Synth fetch params error", e);
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

    const handleModChange = async (index: number, slot: ModSlot) => {
        const newMatrix = [...modMatrix];
        newMatrix[index] = slot;
        setModMatrix(newMatrix);

        if (procIdx !== -1) {
            await invoke("update_mod_matrix", { trackIdx: trackId, procIdx: procIdx, slots: newMatrix });
        }
    };

    const handleLoadPreset = async (path: string) => {
        if (procIdx !== -1) {
            await invoke("load_synth_preset", { trackIdx: trackId, procIdx: procIdx, path });
            fetchParams(); // Refresh UI
        }
    };

    const handleSavePreset = async (name: string) => {
        if (procIdx !== -1) {
            const path = `presets/synth/${name}.json`;
            await invoke("save_synth_preset", { trackIdx: trackId, procIdx: procIdx, path });
        }
    };


    const renderSlider = (paramName: string, label: string, step = 0.01, min?: number, max?: number) => {
        const p = getParam(paramName);
        if (!p) return null;
        return (
            <div className="control-group">
                <label>{label}: {p.value.toFixed(2)}</label>
                <input
                    type="range"
                    min={min !== undefined ? min : p.min_value}
                    max={max !== undefined ? max : p.max_value}
                    step={step}
                    value={p.value}
                    onChange={e => handleParamChange(p.id, parseFloat(e.target.value))}
                />
            </div>
        );
    };

    const renderKnob = (paramName: string, label: string, precision = 2, min?: number, max?: number, specialClass?: string) => {
        // Just a slider for now, styled as a "knob" logic in future
        const p = getParam(paramName);
        if (!p) return null;
        return (
            <div className="control-group compact">
                <label>{label}</label>
                <input
                    type="range"
                    min={min !== undefined ? min : p.min_value}
                    max={max !== undefined ? max : p.max_value}
                    step={0.01}
                    value={p.value}
                    onChange={e => handleParamChange(p.id, parseFloat(e.target.value))}
                    className={specialClass ? specialClass : "knob-slider"}
                />
                <span style={specialClass ? { color: '#ffaa00', fontWeight: 'bold' } : {}}>{p.value.toFixed(precision)}</span>
            </div>
        );
    };

    const renderSelect = (paramName: string, label: string, options: string[]) => {
        const p = getParam(paramName);
        if (!p) return null;
        return (
            <div className="control-group">
                <label>{label}</label>
                <select
                    value={Math.round(p.value)}
                    onChange={e => handleParamChange(p.id, parseFloat(e.target.value))}
                    style={{ padding: '5px', background: '#222', color: '#fff', border: '1px solid #444' }}
                >
                    {options.map((opt, idx) => (
                        <option key={idx} value={idx}>{opt}</option>
                    ))}
                </select>
            </div>
        );
    };

    return (
        <div className="filter-canvas-container" style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', padding: '20px', minHeight: '500px' }}>
            {/* Header / Brand */}
            <div style={{ marginBottom: '20px', textAlign: 'center' }}>
                <h2 style={{ margin: 0, color: '#00ffed', textShadow: '0 0 10px rgba(0,255,237,0.5)' }}>V-ONE SYNTH</h2>
                <span style={{ color: '#666', fontSize: '0.8rem' }}>VIRTUAL ANALOG ENGINE</span>
            </div>

            {/* Tabs */}
            <div style={{ display: 'flex', gap: '10px', marginBottom: '20px', width: '100%', justifyContent: 'center' }}>
                {['osc', 'filter', 'mod', 'global', 'seq'].map(tab => (
                    <button
                        key={tab}
                        onClick={() => setActiveTab(tab as any)}
                        style={{
                            padding: '8px 20px',
                            background: activeTab === tab ? '#00ffed' : '#222',
                            color: activeTab === tab ? '#000' : '#888',
                            border: '1px solid #444',
                            borderRadius: '4px',
                            cursor: 'pointer',
                            fontWeight: 'bold',
                            textTransform: 'uppercase'
                        }}
                    >
                        {tab === 'osc' ? 'Oscillators' : tab === 'filter' ? 'Filter \u0026 Env' : tab === 'mod' ? 'Mod Matrix' : tab === 'global' ? 'Global / FX' : 'Sequencer'}
                    </button>
                ))}
            </div>

            <div style={{ display: 'grid', gridTemplateColumns: 'minmax(200px, 1fr) 3fr', gap: '20px', width: '100%', maxWidth: '1000px' }}>
                <div style={{ height: '600px' }}>
                    <PresetBrowser onLoad={handleLoadPreset} onSave={handleSavePreset} />
                </div>
                <div style={{ width: '100%', background: '#111', padding: '20px', borderRadius: '8px', border: '1px solid #333', minHeight: '600px' }}>

                    {activeTab === 'osc' && (
                        <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
                            {/* OSC 1 */}
                            <div className="section-box" style={{ border: '1px solid #333', padding: '10px', borderRadius: '4px' }}>
                                <h4 style={{ margin: '0 0 10px 0', color: '#00ffed' }}>OSCILLATOR 1 (MASTER)</h4>
                                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px' }}>
                                    {renderSelect("Osc 1 Type", "Waveform", ["Sine", "Saw", "Square", "Triangle", "Noise"])}
                                    {renderSlider("Osc 1 Gain", "Level")}
                                    {renderSlider("Osc 1 Octave", "Octave", 1, -2, 2)}
                                    {renderSlider("Osc 1 Semi", "Semi", 1, -12, 12)}
                                    {renderSlider("Osc 1 Detune", "Detune (ct)", 1)}
                                    {renderKnob("Osc 1 Shape", "Morph / PWM")}

                                    {/* Super Saw Toggle */}
                                    {(() => {
                                        const p = getParam("Super Saw");
                                        if (!p) return null;
                                        const isOn = p.value > 0.5;
                                        return (
                                            <div className="control-group" style={{ gridColumn: 'span 2', flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between', background: '#1a1a1a', padding: '5px', borderRadius: '4px' }}>
                                                <label style={{ color: isOn ? '#00ffed' : '#888' }}>HYPER-STACK (SUPER SAW)</label>
                                                <button
                                                    onClick={() => handleParamChange(p.id, isOn ? 0.0 : 1.0)}
                                                    style={{
                                                        background: isOn ? '#00ffed' : '#333', color: isOn ? '#000' : '#aaa',
                                                        border: 'none', borderRadius: '3px', padding: '4px 12px', fontWeight: 'bold', cursor: 'pointer'
                                                    }}
                                                >
                                                    {isOn ? "ACTIVE" : "OFF"}
                                                </button>
                                            </div>
                                        )
                                    })()}
                                </div>
                            </div>

                            {/* OSC 2 */}
                            <div className="section-box" style={{ border: '1px solid #333', padding: '10px', borderRadius: '4px' }}>
                                <h4 style={{ margin: '0 0 10px 0', color: '#ff00aa' }}>OSCILLATOR 2 (SLAVE)</h4>
                                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px' }}>
                                    {renderSelect("Osc 2 Type", "Waveform", ["Sine", "Saw", "Square", "Triangle", "Noise"])}
                                    {renderSlider("Osc 2 Gain", "Level")}
                                    {renderSlider("Osc 2 Octave", "Octave", 1, -2, 2)}
                                    {renderSlider("Osc 2 Semi", "Semi", 1, -12, 12)}
                                    {renderSlider("Osc 2 Detune", "Detune (ct)", 1)}
                                    {renderKnob("Osc 2 Shape", "Morph / PWM")}
                                    {renderKnob("FM Amount", "Thru-Zero FM", 2, 0, 1, "knob-slider-fm")}
                                </div>
                            </div>

                            {/* OSC 3 / SUB */}
                            <div className="section-box" style={{ border: '1px solid #333', padding: '10px', borderRadius: '4px' }}>
                                <h4 style={{ margin: '0 0 10px 0', color: '#ffff00' }}>OSCILLATOR 3 (SUB)</h4>
                                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px' }}>
                                    {renderSelect("Osc 3 Type", "Waveform", ["Sine", "Saw", "Square", "Triangle", "Noise"])}
                                    {renderSlider("Osc 3 Gain", "Level")}
                                </div>
                            </div>
                        </div>
                    )}

                    {activeTab === 'filter' && (
                        <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
                            {/* FILTER */}
                            <div className="section-box" style={{ border: '1px solid #333', padding: '10px', borderRadius: '4px' }}>
                                <h4 style={{ margin: '0 0 10px 0', color: '#00ffed' }}>VCF (LADDER)</h4>
                                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px' }}>
                                    {renderSelect("Filt Type", "Mode", ["LP 24dB", "HP 24dB", "BP 12dB"])}
                                    {renderSlider("Cutoff", "Cutoff (Hz)", 0, 20, 20000)}
                                    {renderKnob("Resonance", "Res")}
                                    {renderSlider("Keytrack", "KB Track", 2, 0, 2)}
                                    {renderKnob("Env Amount", "Env Amt", 2, -1, 1)}
                                    {renderKnob("Drive", "Drive", 1, 0, 10)}
                                </div>
                            </div>

                            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '20px' }}>
                                {/* AMP ENV */}
                                <div className="section-box" style={{ border: '1px solid #333', padding: '10px', borderRadius: '4px' }}>
                                    <h4 style={{ margin: '0 0 10px 0', color: '#fff' }}>AMP ENV</h4>
                                    <div style={{ marginBottom: '10px', display: 'flex', justifyContent: 'center' }}>
                                        <AdsrGraph
                                            attack={getParam("Amp Atk")?.value || 0}
                                            decay={getParam("Amp Dec")?.value || 0}
                                            sustain={getParam("Amp Sus")?.value || 0}
                                            release={getParam("Amp Rel")?.value || 0}
                                            color="#fff"
                                            width={240}
                                            height={80}
                                            onParamChange={(type, val) => {
                                                const names = { A: "Amp Atk", D: "Amp Dec", S: "Amp Sus", R: "Amp Rel" };
                                                const p = getParam(names[type as keyof typeof names]);
                                                if (p) handleParamChange(p.id, val);
                                            }}
                                        />
                                    </div>
                                    {renderSlider("Amp Atk", "Attack")}
                                    {renderSlider("Amp Dec", "Decay")}
                                    {renderSlider("Amp Sus", "Sustain")}
                                    {renderSlider("Amp Rel", "Release")}
                                </div>

                                {/* FILTER ENV */}
                                <div className="section-box" style={{ border: '1px solid #333', padding: '10px', borderRadius: '4px' }}>
                                    <h4 style={{ margin: '0 0 10px 0', color: '#fff' }}>FILTER ENV</h4>
                                    <div style={{ marginBottom: '10px', display: 'flex', justifyContent: 'center' }}>
                                        <AdsrGraph
                                            attack={getParam("Filt Atk")?.value || 0}
                                            decay={getParam("Filt Dec")?.value || 0}
                                            sustain={getParam("Filt Sus")?.value || 0}
                                            release={getParam("Filt Rel")?.value || 0}
                                            color="#00ffed"
                                            width={240}
                                            height={80}
                                            onParamChange={(type, val) => {
                                                const names = { A: "Filt Atk", D: "Filt Dec", S: "Filt Sus", R: "Filt Rel" };
                                                const p = getParam(names[type as keyof typeof names]);
                                                if (p) handleParamChange(p.id, val);
                                            }}
                                        />
                                    </div>
                                    {renderSlider("Filt Atk", "Attack")}
                                    {renderSlider("Filt Dec", "Decay")}
                                    {renderSlider("Filt Sus", "Sustain")}
                                    {renderSlider("Filt Rel", "Release")}
                                </div>
                            </div>
                        </div>
                    )}

                    {activeTab === 'global' && (
                        <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
                            {/* Dynamic Spectrum Analyzer */}
                            <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr', gap: '20px' }}>
                                <div className="section-box" style={{ padding: '0', borderRadius: '4px', border: '1px solid #333', height: '240px', overflow: 'hidden' }}>
                                    <HolographicScope />
                                </div>
                                <div className="section-box" style={{ padding: '10px', borderRadius: '4px', border: '1px solid #333', height: '240px', overflow: 'hidden' }}>
                                    <SynthXYPad
                                        paramX={getParam("Macro X")}
                                        paramY={getParam("Macro Y")}
                                        labelX="MORPH"
                                        labelY="SPACE"
                                        onUpdate={handleParamChange}
                                    />
                                </div>
                            </div>

                            <div className="section-box" style={{ border: '2px solid #ffaa00', padding: '15px', borderRadius: '8px', background: 'linear-gradient(180deg, rgba(255,170,0,0.1), rgba(0,0,0,0))' }}>
                                <h4 style={{ margin: '0 0 10px 0', color: '#ffaa00', textShadow: '0 0 10px #ffaa00', textAlign: 'center', letterSpacing: '3px' }}>V-ONE CHARACTER</h4>
                                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: '20px', alignItems: 'start' }}>
                                    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center' }}>
                                        {renderKnob("Age/Char", "AGE", 2, undefined, undefined, "knob-slider-age")}
                                        <div style={{ textAlign: 'center', fontSize: '9px', color: '#888', marginTop: '5px' }}>
                                            0%: MODERN<br />50%: VINTAGE<br />100%: BROKEN
                                        </div>
                                    </div>
                                    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center' }}>
                                        {renderKnob("Warmth", "WARMTH")}
                                        <div style={{ textAlign: 'center', fontSize: '9px', color: '#888', marginTop: '5px' }}>
                                            Sat + Bass Boost
                                        </div>
                                    </div>
                                    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center' }}>
                                        {renderKnob("Spread", "WIDTH")}
                                        <div style={{ textAlign: 'center', fontSize: '9px', color: '#888', marginTop: '5px' }}>
                                            Voice Pan Divergence
                                        </div>
                                    </div>
                                </div>
                            </div>

                            <div className="section-box" style={{ border: '1px solid #333', padding: '10px', borderRadius: '4px' }}>
                                <h4 style={{ margin: '0 0 10px 0', color: '#00ffed' }}>LFO (Global)</h4>
                                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px' }}>
                                    {renderSlider("LFO Rate", "Rate (Hz)", 0.1, 0.1, 20.0)}
                                    {renderSlider("LFO Depth", "Depth", 0.01)}
                                </div>
                            </div>

                            {/* FX RACK (Condensed) */}
                            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px' }}>
                                <div className="section-box" style={{ border: '1px solid #333', padding: '5px', borderRadius: '4px' }}>
                                    <h4 style={{ margin: '0 0 5px 0', color: '#ff00aa', fontSize: '11px' }}>CHORUS</h4>
                                    <div style={{ display: 'flex', gap: '5px' }}>
                                        {renderKnob("Chorus Mix", "Mix", 0)}
                                        {renderKnob("Chorus Rate", "Rate", 0)}
                                    </div>
                                </div>
                                <div className="section-box" style={{ border: '1px solid #333', padding: '5px', borderRadius: '4px' }}>
                                    <h4 style={{ margin: '0 0 5px 0', color: '#ffff00', fontSize: '11px' }}>DELAY</h4>
                                    <div style={{ display: 'flex', gap: '5px' }}>
                                        {renderKnob("Delay Mix", "Mix", 0)}
                                        {renderKnob("Delay Time", "Time", 0)}
                                    </div>
                                </div>
                                <div className="section-box" style={{ border: '1px solid #333', padding: '5px', borderRadius: '4px' }}>
                                    <h4 style={{ margin: '0 0 5px 0', color: '#ff5500', fontSize: '11px' }}>DISTORTION</h4>
                                    <div style={{ display: 'flex', gap: '5px' }}>
                                        {renderKnob("Dist Mix", "Mix", 0)}
                                        {renderKnob("Dist Drive", "Drive", 0)}
                                    </div>
                                </div>
                                <div className="section-box" style={{ border: '1px solid #333', padding: '5px', borderRadius: '4px' }}>
                                    <h4 style={{ margin: '0 0 5px 0', color: '#00ffff', fontSize: '11px' }}>REVERB</h4>
                                    <div style={{ display: 'flex', gap: '5px' }}>
                                        {renderKnob("Reverb Mix", "Mix", 0)}
                                        {renderKnob("Reverb Size", "Size", 0)}
                                    </div>
                                </div>
                            </div>

                            <div className="section-box" style={{ border: '1px solid #333', padding: '10px', borderRadius: '4px' }}>
                                <h4 style={{ margin: '0 0 10px 0', color: '#fff' }}>MASTER</h4>
                                {renderSlider("Master Vol", "Master Output")}
                            </div>
                        </div>
                    )}

                    {activeTab === 'seq' && (
                        <div style={{ display: 'grid', gridTemplateColumns: '1fr 2fr', gap: '20px' }}>
                            {/* ARPEGGIATOR */}
                            <div className="section-box" style={{ padding: '20px', border: '1px solid #333', borderRadius: '4px', minHeight: '300px' }}>
                                <h4 style={{ color: '#00ffed', textAlign: 'center', marginBottom: '20px' }}>ARPEGGIATOR</h4>
                                <div style={{ display: 'flex', flexDirection: 'column', gap: '15px', alignItems: 'center' }}>
                                    {renderKnob("Arp On", "ENABLE", 1, 0, 1)}
                                    <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px' }}>
                                        {renderKnob("Arp Mode", "MODE", 0)}
                                        {renderKnob("Arp Rate", "RATE", 2, 0.5, 4)}
                                    </div>
                                    <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px' }}>
                                        {renderKnob("Arp Oct", "OCTAVE", 1, 1, 3)}
                                        {renderKnob("Arp Gate", "GATE", 1)}
                                    </div>
                                    <div style={{ fontSize: '10px', color: '#666', marginTop: '10px', textAlign: 'center' }}>
                                        Mode: 0=Up, 1=Down, 2=Sync, 3=Rnd
                                    </div>
                                </div>
                            </div>

                            {/* STEP SEQUENCER */}
                            <div className="section-box" style={{ padding: '20px', border: '1px solid #333', borderRadius: '4px', minHeight: '300px' }}>
                                <h4 style={{ color: '#00ffed', textAlign: 'center', marginBottom: '10px' }}>STEP LFO SEQUENCER</h4>

                                <div style={{ display: 'flex', justifyContent: 'center', marginBottom: '20px' }}>
                                    <div style={{ textAlign: 'center' }}>
                                        {renderKnob("Seq Target", "Seq Target", 3, 0, 1)}
                                        <div style={{ fontSize: '10px', color: '#666', marginTop: '5px' }}>
                                            Target: Cutoff / Pitch / Res / Vol
                                        </div>
                                    </div>
                                </div>


                                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(8, 1fr)', gap: '10px', height: '200px' }}>
                                    {[1, 2, 3, 4, 5, 6, 7, 8].map(i => {
                                        const p = getParam(`Step ${i}`);
                                        const val = p?.value || 0;
                                        return (
                                            <div key={i} style={{
                                                position: 'relative',
                                                display: 'flex', flexDirection: 'column',
                                                alignItems: 'center', height: '100%',
                                                background: '#222', borderRadius: '4px',
                                                padding: '5px', justifyContent: 'flex-end',
                                                overflow: 'hidden'
                                            }}>
                                                <div style={{
                                                    height: `${val * 100}%`,
                                                    width: '100%',
                                                    background: '#00ffed',
                                                    boxShadow: '0 0 10px #00ffed',
                                                    borderRadius: '2px',
                                                    pointerEvents: 'none',
                                                    transition: 'height 0.1s'
                                                }} />
                                                {/* Invisible Overlay Input */}
                                                <input
                                                    type="range"
                                                    min="0" max="1" step="0.05"
                                                    value={val}
                                                    onChange={(e) => handleParamChange(p?.id!, parseFloat(e.target.value))}
                                                    style={{
                                                        position: 'absolute',
                                                        left: 0, bottom: 0,
                                                        height: '200px', // Taller than container to cover it
                                                        width: '100%',
                                                        opacity: 0,
                                                        cursor: 'ns-resize',
                                                        margin: 0
                                                    }}
                                                />
                                                <span style={{ marginTop: '5px', color: '#888', fontSize: '12px', zIndex: 2 }}>{i}</span>
                                            </div>
                                        )
                                    })}
                                </div>
                                <div style={{ marginTop: '20px', textAlign: 'center', color: '#555', fontSize: '11px', borderTop: '1px solid #333', paddingTop: '10px' }}>
                                    <p>Parameter Locking: Modulates target based on LFO Rate phase.</p>
                                </div>
                            </div>
                        </div>
                    )}

                    {activeTab === 'mod' && (
                        <div style={{ width: '100%', height: '400px' }}>
                            <ModMatrix slots={modMatrix} onChange={handleModChange} />
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
};
