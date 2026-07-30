import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './MagnetoSettings.css';

interface ParameterInfo {
    id: string;
    name: string;
    value: number;
    min_value: number;
    max_value: number;
}

interface EffectInfo {
    id: string;
    name: string;
    parameters: ParameterInfo[];
}

export const MagnetoSettings: React.FC = () => {
    const [masterEffects, setMasterEffects] = useState<EffectInfo[]>([]);

    const fetchMasterInfo = async () => {
        try {
            const info = await invoke<EffectInfo[]>("get_master_info");
            setMasterEffects(info);
        } catch (e) {
            console.error("Failed to fetch master info", e);
        }
    };

    useEffect(() => {
        fetchMasterInfo();
        const interval = setInterval(fetchMasterInfo, 200);
        return () => clearInterval(interval);
    }, []);

    const handleParamChange = async (paramId: string, value: number) => {
        await invoke("set_parameter", { paramId, value });
        setMasterEffects(prev => prev.map(fx => ({
            ...fx,
            parameters: fx.parameters.map(p => p.id === paramId ? { ...p, value } : p)
        })));
    };

    return (
        <div className="magneto-master-panel glass">
            <div className="magneto-header">
                <h2>MAGNETO-GRAWITACJA V1</h2>
                <span className="magneto-status">QUANTUM-LINK ACTIVE</span>
            </div>
            <div className="magneto-grid">
                {masterEffects.map(fx => (
                    <div key={fx.id} className="magneto-fx-unit">
                        <h3>{fx.name}</h3>
                        <div className="magneto-params">
                            {fx.parameters.map(p => (
                                <div key={p.id} className="magneto-param-row">
                                    <div className="magneto-param-label">
                                        <span>{p.name}</span>
                                        <span className="magneto-param-value">
                                            {p.name.includes("Crosstalk")
                                                ? `${(p.value * 1000).toFixed(1)} mU`
                                                : p.name === "Master Dither"
                                                    ? (p.value > 0.5 ? "ON" : "OFF")
                                                    : p.value.toFixed(2)}
                                        </span>
                                    </div>
                                    <input
                                        type="range"
                                        className="magneto-slider"
                                        min={p.min_value}
                                        max={p.max_value}
                                        step={p.name === "Master Dither" ? "1" : "0.0001"}
                                        value={p.value}
                                        onChange={(e) => handleParamChange(p.id, parseFloat(e.target.value))}
                                    />
                                    {p.name === "Master Dither" && (
                                        <button
                                            className={`btn-magneto-toggle ${p.value > 0.5 ? 'active' : ''}`}
                                            onClick={() => handleParamChange(p.id, p.value > 0.5 ? 0 : 1)}
                                        >
                                            {p.value > 0.5 ? 'DITHERING ACTIVE' : 'NO DITHER'}
                                        </button>
                                    )}
                                </div>
                            ))}
                        </div>
                    </div>
                ))}
            </div>
        </div>
    );
};
