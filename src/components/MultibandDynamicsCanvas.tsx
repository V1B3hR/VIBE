import React, { useEffect, useRef, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './MultibandDynamicsCanvas.css';

interface MultibandDynamicsCanvasProps {
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

const BAND_NAMES = ["Low", "LowMid", "HighMid", "High"];
const FREQ_COLORS = ["#00f0ff", "#ff0055", "#ffaa00", "#aaff00"];

export const MultibandDynamicsCanvas: React.FC<MultibandDynamicsCanvasProps> = ({ trackId, processorId }) => {
    const [params, setParams] = useState<Parameter[]>([]);
    const vizCanvasRef = useRef<HTMLCanvasElement>(null);
    const [kropelkaActive, setKropelkaActive] = useState(false);
    const [hoveredBand, setHoveredBand] = useState<number | null>(null);

    const fetchParams = useCallback(async () => {
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
            console.error("MB fetch params error", e);
        }
    }, [trackId, processorId]);

    useEffect(() => {
        fetchParams();
        const interval = setInterval(fetchParams, 100);
        return () => clearInterval(interval);
    }, [fetchParams]);

    const handleParamChange = async (id: string, value: number) => {
        await invoke("set_parameter", { paramId: id, value });
        setParams(prev => prev.map(p => p.id === id ? { ...p, value } : p));
    };

    const handleDoubleClickReset = (p: Parameter) => {
        let defaultVal = (p.min_value + p.max_value) / 2;
        const nameLower = p.name.toLowerCase();
        if (nameLower.includes("thr")) {
            defaultVal = -20.0;
        } else if (nameLower.includes("ratio")) {
            defaultVal = 2.0;
        } else if (nameLower.includes("gain")) {
            defaultVal = 0.0;
        } else if (p.name.includes("XOver 1")) {
            defaultVal = 120.0;
        } else if (p.name.includes("XOver 2")) {
            defaultVal = 1200.0;
        } else if (p.name.includes("XOver 3")) {
            defaultVal = 6000.0;
        }
        handleParamChange(p.id, defaultVal);
    };

    const getParam = (nameStart: string) => params.find(p => p.name.startsWith(nameStart));

    // Kropelka Auto-Tame 'The Mud' Action
    const handleTameMud = async () => {
        setKropelkaActive(true);
        // Find LowMid threshold and ratio, then adjust them automatically
        const lmThr = params.find(p => p.name === "LowMid Thr");
        const lmRatio = params.find(p => p.name === "LowMid Ratio");
        const lmGain = params.find(p => p.name === "LowMid Gain");

        if (lmThr) await handleParamChange(lmThr.id, -18.0);
        if (lmRatio) await handleParamChange(lmRatio.id, 4.0);
        if (lmGain) await handleParamChange(lmGain.id, -2.5); // cut the mud

        setTimeout(() => setKropelkaActive(false), 2000);
    };

    // Draw Intuitive Crossover & Dynamics Viz
    useEffect(() => {
        const canvas = vizCanvasRef.current;
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        const w = canvas.width;
        const h = canvas.height;

        ctx.clearRect(0, 0, w, h);

        const xovers = params.filter(p => p.name.includes("XOver")).map(p => p.value);
        if (xovers.length < 3) return; // Wait for data

        const freqToX = (f: number) => {
            const logMin = Math.log10(20);
            const logMax = Math.log10(20000);
            return ((Math.log10(Math.max(20, Math.min(20000, f))) - logMin) / (logMax - logMin)) * w;
        };

        const boundaries = [20, ...xovers, 20000];

        // Draw Interactive Background Regions
        for (let i = 0; i < 4; i++) {
            const startX = freqToX(boundaries[i]);
            const endX = freqToX(boundaries[i + 1]);
            const bandW = Math.max(0, endX - startX);

            const isHovered = hoveredBand === i;
            const isMud = i === 1; // LowMid is "The Mud"

            const grad = ctx.createLinearGradient(0, h, 0, 0);
            grad.addColorStop(0, `${FREQ_COLORS[i]}` + (isHovered ? '44' : '15'));
            grad.addColorStop(1, `${FREQ_COLORS[i]}` + (isHovered ? '77' : '33'));

            ctx.fillStyle = grad;
            ctx.fillRect(startX, 0, bandW, h);

            // Draw Kropelka Mud Diagnostics
            if (isMud) {
                const mudThr = getParam("LowMid Thr")?.value || 0;
                const limitY = h / 2 - (mudThr * 2);

                ctx.save();
                ctx.beginPath();
                ctx.rect(startX, 0, bandW, limitY);
                ctx.clip();
                
                // Draw diagonal warning lines in the mud suppression zone
                ctx.strokeStyle = '#ff0055';
                ctx.lineWidth = 1;
                ctx.globalAlpha = kropelkaActive ? 0.8 : 0.3;
                for (let d = -h; d < w; d += 10) {
                    ctx.beginPath();
                    ctx.moveTo(startX + d, 0);
                    ctx.lineTo(startX + d + h, h);
                    ctx.stroke();
                }
                ctx.restore();

                // Alert text
                ctx.fillStyle = kropelkaActive ? '#fff' : '#ff0055';
                ctx.globalAlpha = kropelkaActive ? 1.0 : 0.6;
                ctx.font = 'bold 10px SF Mono';
                ctx.textAlign = 'center';
                ctx.fillText(kropelkaActive ? "TAMING THE MUD..." : "THE MUD ZONE", startX + bandW/2, limitY - 10);
                ctx.globalAlpha = 1.0;
            }

            // Draw Dynamics Curve for this band
            const thr = getParam(`${BAND_NAMES[i]} Thr`)?.value || 0;
            const ratio = getParam(`${BAND_NAMES[i]} Ratio`)?.value || 1;
            const gain = getParam(`${BAND_NAMES[i]} Gain`)?.value || 0;
            
            ctx.beginPath();
            ctx.strokeStyle = FREQ_COLORS[i];
            ctx.lineWidth = 2;
            ctx.lineJoin = 'round';

            const startY = h / 2 - (gain * 2);
            ctx.moveTo(startX, startY);

            // Knee point
            const kneeX = startX + bandW * 0.7; // Symbolic representation of hitting threshold
            const kneeY = h / 2 - ((thr + gain) * 2);
            ctx.lineTo(kneeX, kneeY);

            // Compressed path
            const compY = kneeY + (20 / ratio);
            ctx.lineTo(endX, compY);
            
            ctx.stroke();

            // Band Labels
            ctx.fillStyle = '#fff';
            ctx.font = '10px sans-serif';
            ctx.textAlign = 'left';
            ctx.fillText(BAND_NAMES[i], startX + 10, 20);
        }

        // Draw Crossover dividers
        ctx.setLineDash([4, 4]);
        ctx.lineWidth = 1;
        xovers.forEach((f, idx) => {
            const x = freqToX(f);
            ctx.beginPath();
            
            // XOver handles
            ctx.strokeStyle = '#aaa';
            ctx.moveTo(x, 0);
            ctx.lineTo(x, h);
            ctx.stroke();

            // Bubble
            ctx.fillStyle = '#111';
            ctx.strokeStyle = FREQ_COLORS[idx+1];
            ctx.setLineDash([]);
            ctx.beginPath();
            ctx.roundRect(x - 20, h - 25, 40, 15, 4);
            ctx.fill();
            ctx.stroke();
            
            ctx.fillStyle = '#fff';
            ctx.textAlign = 'center';
            ctx.fillText(f.toFixed(0), x, h - 14);
            ctx.setLineDash([4, 4]);
        });
        ctx.setLineDash([]);

    }, [params, hoveredBand, kropelkaActive]);

    const handleCanvasMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
        const canvas = vizCanvasRef.current;
        if (!canvas) return;
        
        const rect = canvas.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const w = rect.width;

        const xovers = params.filter(p => p.name.includes("XOver")).map(p => p.value);
        if (xovers.length < 3) return;

        const freqToX = (f: number) => {
            const logMin = Math.log10(20);
            const logMax = Math.log10(20000);
            return ((Math.log10(Math.max(20, Math.min(20000, f))) - logMin) / (logMax - logMin)) * w;
        };

        const b1 = freqToX(xovers[0]);
        const b2 = freqToX(xovers[1]);
        const b3 = freqToX(xovers[2]);

        if (x < b1) setHoveredBand(0);
        else if (x >= b1 && x < b2) setHoveredBand(1);
        else if (x >= b2 && x < b3) setHoveredBand(2);
        else setHoveredBand(3);
    };

    const handleCanvasMouseLeave = () => {
        setHoveredBand(null);
    };

    const renderDial = (p: Parameter, suffix: string = "") => {
        if (!p) return null;
        const norm = (p.value - p.min_value) / (p.max_value - p.min_value);
        const rotation = -135 + (norm * 270);
        
        return (
            <div className="mb-dial" key={p.id}>
                <div className="dial-graphic" style={{ transform: `rotate(${rotation}deg)` }}>
                    <div className="dial-pointer"></div>
                </div>
                <input
                    type="range"
                    min={p.min_value}
                    max={p.max_value}
                    step={p.max_value > 20 ? 1 : 0.1}
                    value={p.value}
                    onChange={(e) => handleParamChange(p.id, parseFloat(e.target.value))}
                    onDoubleClick={() => handleDoubleClickReset(p)}
                    className="dial-hidden-input"
                    title={p.name}
                />
                <div className="dial-value">{p.value.toFixed(1)}{suffix}</div>
                <div className="dial-label">{p.name.split(" ").slice(1).join(" ")}</div>
            </div>
        );
    };

    return (
        <div className="mb-dynamics-temple">
            <div className="mb-header">
                <h3>VIBE MULTI-BAND <span className="mb-pro-badge">PRO FX</span></h3>
                <div className="kropelka-assist-container">
                    <button 
                        className={`kropelka-assist-btn ${kropelkaActive ? 'active' : ''}`}
                        onClick={handleTameMud}
                    >
                        <span className="kropelka-avatar">💧</span>
                        Tame The Mud
                    </button>
                    <div className="assist-tooltip">
                        Kropelka will analyze the 200-500Hz region and apply smart dynamic suppression to tighten the mix.
                    </div>
                </div>
            </div>

            <div className="mb-viz-stage">
                <canvas 
                    ref={vizCanvasRef} 
                    width={700} height={200} 
                    className="mb-reactive-canvas"
                    onMouseMove={handleCanvasMouseMove}
                    onMouseLeave={handleCanvasMouseLeave}
                />
            </div>

            <div className="mb-bands-grid">
                {[0, 1, 2, 3].map(i => {
                    const bandName = BAND_NAMES[i];
                    return (
                        <div key={i} className={`mb-band-channel ${hoveredBand === i ? 'hovered' : ''}`} style={{ borderColor: hoveredBand === i ? FREQ_COLORS[i] : 'rgba(255,255,255,0.05)' }}>
                            <div className="band-name" style={{ color: FREQ_COLORS[i] }}>{bandName.toUpperCase()}</div>
                            <div className="dials-grid">
                                {renderDial(params.find(p => p.name === `${bandName} Thr`)!, "dB")}
                                {renderDial(params.find(p => p.name === `${bandName} Ratio`)!, ":1")}
                                {renderDial(params.find(p => p.name === `${bandName} Atk`)!, "ms")}
                                {renderDial(params.find(p => p.name === `${bandName} Rel`)!, "ms")}
                                <div style={{ gridColumn: 'span 2' }}>
                                    {renderDial(params.find(p => p.name === `${bandName} Gain`)!, "dB")}
                                </div>
                            </div>
                        </div>
                    );
                })}
            </div>
        </div>
    );
};
