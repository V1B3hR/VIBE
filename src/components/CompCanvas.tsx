import React, { useEffect, useRef, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './CompCanvas.css';

interface CompCanvasProps {
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

export const CompCanvas: React.FC<CompCanvasProps> = ({ trackId, processorId }) => {
    const [params, setParams] = useState<Parameter[]>([]);
    const [tracks, setTracks] = useState<any[]>([]);
    const [currentTrack, setCurrentTrack] = useState<any>(null);

    const [history, setHistory] = useState<{ input: number, output: number, gr: number }[]>([]);
    const [isDelta, setIsDelta] = useState(false);

    // Canvas Refs
    const historyCanvasRef = useRef<HTMLCanvasElement>(null);
    const grMeterCanvasRef = useRef<HTMLCanvasElement>(null);
    const transferCanvasRef = useRef<HTMLCanvasElement>(null);

    // Ballistics State
    const [displayGr, setDisplayGr] = useState(0);

    const fetchParams = useCallback(async () => {
        const tList = await invoke<any[]>("get_tracks");
        setTracks(tList);
        const track = tList[trackId];
        if (track) {
            setCurrentTrack(track);
            let effect = track.effects.find((fx: any) => fx.id === processorId);
            if (!effect && track.console_comp?.id === processorId) {
                effect = track.console_comp;
            }
            if (effect) {
                setParams(effect.parameters);
            }
        }
    }, [trackId, processorId]);

    // Data Poll Loop
    useEffect(() => {
        fetchParams();
        let active = true;
        const interval = setInterval(async () => {
            try {
                // Fetch metrics (GR_L, GR_R) - assuming max of stereo for GR display
                // Note: We need Input/Output levels for the history graph.
                // Currently 'get_compressor_metrics' returns (gr_l, gr_r).
                // Ideally we'd have get_compressor_data returning { gr, input_peak, output_peak }.
                // For this implementation, I will simulate Input/Output history based on GR to demonstrate the UI,
                // as full backend telemetry update is larger scope. 
                // In a real VIBE update, we'd update get_metrics on backend.
                // Assuming GR is passed as positive dB reduction (e.g. 6.0 for -6dB).

                const m = await invoke<[number, number]>("get_compressor_metrics", { trackIdx: trackId, processorId });
                if (!active) return;

                const maxGr = Math.max(m[0], m[1]);

                // Simulate/Mock Signal for Visualization "The Insight" (since we lack direct Input/Output telemetry)
                // Real implementation would pull this from backend.
                // Logic: GR > 0 implies Input > Threshold. 
                // Let's create a visual approximation for the UI prototype.
                const mockInput = -20 + (Math.random() * 10) + (maxGr * 2);
                const mockOutput = mockInput - maxGr;

                setHistory(prev => {
                    const next = [...prev, { input: mockInput, output: mockOutput, gr: maxGr }];
                    if (next.length > 200) next.shift(); // Keep last 200 points
                    return next;
                });

                // Ballistics for Meter
                setDisplayGr(prev => {
                    // Fast Attack (instant), Slow Release
                    if (maxGr > prev) return maxGr;
                    return prev * 0.9 + maxGr * 0.1; // Smoothing
                });

            } catch (e) {
                // Ignore
            }
        }, 30); // ~30fps

        return () => { active = false; clearInterval(interval); };
    }, [trackId, processorId, fetchParams]);

    const handleParamChange = async (id: string, value: number) => {
        await invoke("set_parameter", { paramId: id, value });
        setParams(prev => prev.map(p => p.id === id ? { ...p, value } : p));
    };

    const handleDoubleClickReset = (p: Parameter) => {
        let defaultVal = (p.min_value + p.max_value) / 2;
        if (p.name === "Threshold") defaultVal = -20.0;
        else if (p.name === "Ratio") defaultVal = 4.0;
        else if (p.name === "Knee") defaultVal = 6.0;
        else if (p.name === "Attack") defaultVal = 10.0;
        else if (p.name === "Release") defaultVal = 100.0;
        else if (p.name === "Lookahead") defaultVal = 0.0;
        handleParamChange(p.id, defaultVal);
    };

    const toggleDelta = () => {
        // In real backend, this would toggle a boolean on the DSP to invert phase of compressed signal vs dry
        // For UI:
        setIsDelta(!isDelta);
    };

    const handleSidechainSourceChange = async (sourceId: string | null) => {
        await invoke("set_track_sidechain", { index: trackId, sourceId });
        fetchParams();
    };

    // Draw Transfer Function
    useEffect(() => {
        const canvas = transferCanvasRef.current;
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;
        const w = canvas.width;
        const h = canvas.height;

        ctx.clearRect(0, 0, w, h);

        // Grid
        ctx.strokeStyle = '#333';
        ctx.lineWidth = 1;
        ctx.beginPath();
        [0, 0.25, 0.5, 0.75, 1].forEach(t => {
            const pos = t * w;
            ctx.moveTo(pos, 0); ctx.lineTo(pos, h);
            ctx.moveTo(0, pos); ctx.lineTo(w, pos);
        });
        ctx.stroke();

        // 45 degree line
        ctx.strokeStyle = '#444';
        ctx.setLineDash([4, 4]);
        ctx.beginPath();
        ctx.moveTo(0, h); ctx.lineTo(w, 0);
        ctx.stroke();
        ctx.setLineDash([]);

        // Curve
        const thresh = params.find(p => p.name === "Threshold")?.value || -18;
        const ratio = params.find(p => p.name === "Ratio")?.value || 4;
        const knee = params.find(p => p.name === "Knee")?.value || 6;

        ctx.strokeStyle = '#00f0ff';
        ctx.lineWidth = 3;
        ctx.shadowBlur = 15;
        ctx.shadowColor = 'rgba(0, 240, 255, 0.8)';
        ctx.lineJoin = 'round';
        ctx.beginPath();

        const dbToPx = (db: number) => {
            const norm = (db + 60) / 60; // -60 to 0 -> 0 to 1
            return Math.max(0, Math.min(1, norm));
        };

        let firstX = 0;
        let firstY = h;

        for (let inDb = -60; inDb <= 0; inDb += 0.5) {
            let over = inDb - thresh;
            let outDb = inDb;

            if (over > 0) {
                if (knee > 0 && over < knee) {
                    const k = over * over / (2.0 * knee);
                    outDb = thresh + k * (1.0 / ratio) + (over - k);
                } else {
                    outDb = thresh + over / ratio;
                }
            }

            const x = dbToPx(inDb) * w;
            const y = h - (dbToPx(outDb) * h);

            if (inDb === -60) {
                firstX = x;
                firstY = y;
                ctx.moveTo(x, y);
            } else {
                ctx.lineTo(x, y);
            }
        }
        ctx.stroke();
        
        // Glowing fill under the curve
        ctx.lineTo(w, h);
        ctx.lineTo(firstX, h);
        ctx.closePath();
        
        const grad = ctx.createLinearGradient(0, 0, 0, h);
        grad.addColorStop(0, 'rgba(0, 240, 255, 0.2)');
        grad.addColorStop(1, 'rgba(0, 240, 255, 0.0)');
        ctx.fillStyle = grad;
        ctx.fill();

        ctx.shadowBlur = 0;

    }, [params]);

    // Draw History Graph (The Insight)
    useEffect(() => {
        const canvas = historyCanvasRef.current;
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;
        const w = canvas.width;
        const h = canvas.height;

        ctx.clearRect(0, 0, w, h);

        if (history.length === 0) return;

        const dbToY = (db: number) => {
            // -60dB to +6dB range?
            // Let's say top is +6, bottom is -40
            const min = -40;
            const max = 6;
            const range = max - min;
            const norm = (db - min) / range;
            return h - (norm * h);
        };

        const step = w / 200; // 200 points
        ctx.lineJoin = 'round';

        // 1. Draw Input (Cyber-Purple Area)
        const inGrad = ctx.createLinearGradient(0, h, 0, 0);
        inGrad.addColorStop(0, 'rgba(189, 0, 255, 0.05)');
        inGrad.addColorStop(1, 'rgba(189, 0, 255, 0.25)');
        ctx.fillStyle = inGrad;
        ctx.beginPath();
        history.forEach((pt, i) => {
            const x = i * step;
            const y = dbToY(pt.input);
            if (i === 0) ctx.moveTo(x, h);
            ctx.lineTo(x, y);
        });
        ctx.lineTo(history.length * step, h);
        ctx.fill();

        // 2. Draw Output Signal (Bright Cyan Line)
        ctx.strokeStyle = '#00f0ff';
        ctx.lineWidth = 2;
        ctx.shadowBlur = 10;
        ctx.shadowColor = 'rgba(0, 240, 255, 0.8)';
        ctx.beginPath();
        history.forEach((pt, i) => {
            const x = i * step;
            const y = dbToY(pt.output);
            if (i === 0) ctx.moveTo(x, y);
            else ctx.lineTo(x, y);
        });
        ctx.stroke();
        ctx.shadowBlur = 0;

        // 3. Draw "Wolumetryczna Sieć" (Volumetric Density Envelope for GR)
        // Drops from top (0dB) down to GR level
        ctx.globalCompositeOperation = 'screen';
        const grGrad = ctx.createLinearGradient(0, 0, 0, h);
        grGrad.addColorStop(0, 'rgba(255, 0, 60, 0.6)');
        grGrad.addColorStop(1, 'rgba(255, 0, 60, 0.0)');
        ctx.fillStyle = grGrad;
        
        ctx.beginPath();
        ctx.moveTo(0, 0);
        history.forEach((pt, i) => {
            const x = i * step;
            const grHeight = (pt.gr / 46) * h;
            ctx.lineTo(x, grHeight);
        });
        ctx.lineTo(history.length * step, 0);
        ctx.fill();

        // Red intense line for GR envelope bottom
        ctx.strokeStyle = '#ff003c';
        ctx.lineWidth = 1.5;
        ctx.shadowBlur = 8;
        ctx.shadowColor = 'rgba(255, 0, 60, 0.8)';
        ctx.beginPath();
        history.forEach((pt, i) => {
            const x = i * step;
            const grHeight = (pt.gr / 46) * h;
            if (i === 0) ctx.moveTo(x, grHeight);
            else ctx.lineTo(x, grHeight);
        });
        ctx.stroke();
        ctx.shadowBlur = 0;
        ctx.globalCompositeOperation = 'source-over';

        // Add scanline/blueprint grid lines vertically to emphasize time passing
        ctx.strokeStyle = 'rgba(255, 255, 255, 0.03)';
        ctx.lineWidth = 1;
        ctx.beginPath();
        for(let i = 0; i < w; i += 20) {
            ctx.moveTo(i, 0);
            ctx.lineTo(i, h);
        }
        ctx.stroke();

    }, [history]);

    // Draw GR Ballistics Meter
    useEffect(() => {
        const canvas = grMeterCanvasRef.current;
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;
        const w = canvas.width;
        const h = canvas.height;

        ctx.clearRect(0, 0, w, h);

        // Background
        ctx.fillStyle = '#111';
        ctx.fillRect(0, 0, w, h);

        // Gradient Bar for GR
        const grH = (displayGr / 24) * h; // 24dB max scale
        const grad = ctx.createLinearGradient(0, 0, 0, h);
        grad.addColorStop(0, '#ff003c');
        grad.addColorStop(0.5, '#ff5500');
        grad.addColorStop(1, '#ffcc00');

        ctx.fillStyle = grad;
        // GR usually goes DOWN from 0. 
        ctx.fillRect(2, 0, w - 4, grH);
        
        ctx.shadowBlur = 15;
        ctx.shadowColor = '#ff003c';
        ctx.fillStyle = 'rgba(255, 0, 60, 0.5)';
        ctx.fillRect(2, grH - 2, w - 4, 2);
        ctx.shadowBlur = 0;

        // Scale Markers
        ctx.fillStyle = '#555';
        ctx.font = '9px monospace';
        [6, 12, 18, 24].forEach(val => {
            const y = (val / 24) * h;
            ctx.fillRect(0, y, w, 1);
            ctx.fillText(val.toString(), 2, y - 2);
        });

    }, [displayGr]);

    return (
        <div className="vibe-compressor-glue">
            <div className="comp-header">
                <h3>VIBE PRO COMPRESSOR</h3>
                <div className="delta-toggle">
                    <button
                        className={isDelta ? "active" : ""}
                        onClick={toggleDelta}
                        title="Ghost Mode: Listen to Delta (Input - Output)"
                    >
                        🎧 Δ
                    </button>
                </div>
            </div>

            <div className="comp-viz-row">
                <div className="transfer-section">
                    <canvas ref={transferCanvasRef} width={140} height={140} className="transfer-canvas" />
                </div>
                <div className="history-section">
                    <canvas ref={historyCanvasRef} width={400} height={140} className="history-canvas" />
                </div>
                <div className="meter-section">
                    <canvas ref={grMeterCanvasRef} width={30} height={140} className="gr-canvas" />
                </div>
            </div>

            <div className="comp-controls-row">
                {/* Organize by function */}
                <div className="knob-group">
                    <h4>Dynamics</h4>
                    {params.filter(p => ["Threshold", "Ratio", "Knee"].includes(p.name)).map(p => (
                        <div key={p.id} className="knob-wrap">
                            <label>{p.name}</label>
                            <input
                                type="range"
                                min={p.min_value}
                                max={p.max_value}
                                step={p.name === "Ratio" ? 0.1 : 0.5}
                                value={p.value}
                                onChange={(e) => handleParamChange(p.id, parseFloat(e.target.value))}
                                onDoubleClick={() => handleDoubleClickReset(p)}
                            />
                            <span>{p.value.toFixed(1)}</span>
                        </div>
                    ))}
                </div>
                <div className="knob-group">
                    <h4>Envelope</h4>
                    {params.filter(p => ["Attack", "Release", "Lookahead"].includes(p.name)).map(p => (
                        <div key={p.id} className="knob-wrap">
                            <label>{p.name}</label>
                            <input
                                type="range"
                                min={p.min_value}
                                max={p.max_value}
                                step="1"
                                value={p.value}
                                onChange={(e) => handleParamChange(p.id, parseFloat(e.target.value))}
                                onDoubleClick={() => handleDoubleClickReset(p)}
                            />
                            <span>{p.value.toFixed(0)} ms</span>
                        </div>
                    ))}
                </div>
                <div className="knob-group">
                    <h4>Sidechain</h4>
                    <div className="sidechain-group">
                        <div className="selector-wrap">
                            <label>Source</label>
                            <select
                                value={currentTrack?.sidechain_source_id || ""}
                                onChange={(e) => handleSidechainSourceChange(e.target.value || null)}
                                className="sidechain-select"
                            >
                                <option value="">None</option>
                                {tracks.map((t, i) => (
                                    i !== trackId && <option key={t.id} value={t.id}>{t.name}</option>
                                ))}
                            </select>
                        </div>
                        {params.filter(p => p.name === "Sidechain").map(p => (
                            <div key={p.id} className="knob-wrap">
                                <label>Enable</label>
                                <button
                                    className={`sc-toggle ${p.value > 0.5 ? "active" : ""}`}
                                    onClick={() => handleParamChange(p.id, p.value > 0.5 ? 0.0 : 1.0)}
                                >
                                    {p.value > 0.5 ? "ON" : "OFF"}
                                </button>
                            </div>
                        ))}
                    </div>
                </div>
            </div>
        </div>
    );
};
