import React, { useState, useEffect, useRef, useCallback } from 'react';
import { ParamKnob } from './ParamKnob';
import { PluginPresetBrowser } from './PluginPresetBrowser';
import './PluginRackUnit.css';
import { invoke } from '@tauri-apps/api/core';

interface Parameter {
    id: string;
    name: string;
    value: number;
    min_value: number;
    max_value: number;
}

interface Effect {
    id: string;
    name: string;
    is_bypassed: boolean;
    parameters: Parameter[];
}

interface PluginRackUnitProps {
    effect: Effect;
    trackIndex: number;
    effectIndex: number;
    onBypassToggle: (trackIdx: number, effectId: string, currentBypass: boolean) => void;
    onParamChange: (paramId: string, value: number) => void;
    onOpenEditor?: (trackIdx: number, effectId: string, type: string) => void;
    onRemove?: (trackIdx: number, effectId: string) => void;
    onDragStart: (e: React.DragEvent) => void;
    onDrop: (e: React.DragEvent) => void;
}

interface ContextMenu {
    x: number;
    y: number;
}

// Native VIBE plugins that have custom GUI editors
const NATIVE_PLUGINS = new Set([
    "Prisma EQ", "Vibe Compressor", "Magneto-Tube Limiter",
    "VOne Synth", "VIBE Filter", "VIBE Reverb", "VIBE Delay",
    "VIBE Saturation", "Frenzy Multiplier", "Convolution Reverb",
    "Multiband Dynamics", "Spectral Gate", "Stereo Imager",
]);

export const PluginRackUnit: React.FC<PluginRackUnitProps> = ({
    effect,
    trackIndex,
    effectIndex,
    onBypassToggle,
    onParamChange,
    onOpenEditor,
    onRemove,
    onDragStart,
    onDrop
}) => {
    const isNative = NATIVE_PLUGINS.has(effect.name);
    const [showPresets, setShowPresets] = useState(false);
    const [ctxMenu, setCtxMenu] = useState<ContextMenu | null>(null);
    const ctxRef = useRef<HTMLDivElement>(null);
    const [cpuUsage, setCpuUsage] = useState(0);
    const [factoryPrograms, setFactoryPrograms] = useState<string[]>([]);
    const [showPrograms, setShowPrograms] = useState(false);

    // Close context menu on outside click
    useEffect(() => {
        if (!ctxMenu) return;
        const close = (e: MouseEvent) => {
            if (ctxRef.current && !ctxRef.current.contains(e.target as Node)) {
                setCtxMenu(null);
            }
        };
        document.addEventListener('mousedown', close);
        return () => document.removeEventListener('mousedown', close);
    }, [ctxMenu]);

    const handleContextMenu = useCallback((e: React.MouseEvent) => {
        e.preventDefault();
        e.stopPropagation();
        setCtxMenu({ x: e.clientX, y: e.clientY });
    }, []);

    const closeCtx = () => setCtxMenu(null);

    // Bidirectional Sync: Poll for parameter changes, CPU, and programs
    useEffect(() => {
        if (isNative) return;

        // Fetch programs once
        invoke<string[]>('get_plugin_programs', { trackIdx: trackIndex, processorId: effect.id })
            .then(setFactoryPrograms)
            .catch(() => { });

        const interval = setInterval(async () => {
            try {
                // Poll param changes
                const changes = await invoke<[string, number][]>('poll_plugin_param_changes', {
                    trackIdx: trackIndex,
                    processorId: effect.id
                });

                if (changes && changes.length > 0) {
                    for (const [uuid, value] of changes) {
                        if (uuid !== "unknown") {
                            onParamChange(uuid, value);
                        }
                    }
                }

                // Poll CPU usage
                const cpu = await invoke<number>('get_plugin_cpu_usage', {
                    trackIdx: trackIndex,
                    processorId: effect.id
                });
                setCpuUsage(cpu);

            } catch (e) {
                // Background polling errors are silent
            }
        }, 100);
        return () => clearInterval(interval);
    }, [isNative, trackIndex, effect.id, onParamChange]);

    const handleProgramChange = async (idx: number) => {
        await invoke('set_plugin_program', {
            trackIdx: trackIndex,
            processorId: effect.id,
            programIdx: idx
        });
        setShowPrograms(false);
    };

    return (
        <div
            className={`plugin-rack-unit ${effect.is_bypassed ? 'bypassed' : ''}`}
            draggable
            onDragStart={onDragStart}
            onDragOver={(e) => e.preventDefault()}
            onDrop={onDrop}
            onContextMenu={handleContextMenu}
        >
            <div className="rack-header">
                <div className="rack-title-group">
                    <input
                        type="checkbox"
                        checked={!effect.is_bypassed}
                        onChange={() => onBypassToggle(trackIndex, effect.id, effect.is_bypassed)}
                        className="power-toggle"
                        title={effect.is_bypassed ? 'Enable' : 'Bypass'}
                    />
                    <span className="rack-name" title={effect.name}>{effect.name}</span>
                </div>
                <div className="rack-actions">
                    {/* Preset browser */}
                    <button
                        className="btn-rack-presets"
                        onClick={() => setShowPresets(v => !v)}
                        title="Plugin Presets"
                    >
                        📋
                    </button>
                    {/* CPU Meter */}
                    <div className="cpu-meter-container" title={`CPU: ${(cpuUsage * 100).toFixed(1)}%`}>
                        <div
                            className="cpu-meter-fill"
                            style={{
                                width: `${Math.min(100, cpuUsage * 100)}%`,
                                backgroundColor: cpuUsage > 0.7 ? '#ff4d4d' : cpuUsage > 0.4 ? '#ffca28' : '#2ecc71'
                            }}
                        />
                    </div>
                    {/* Factory Programs */}
                    {factoryPrograms.length > 0 && (
                        <div className="program-select-trigger" onClick={() => setShowPrograms(v => !v)}>
                            <span className="icon-preset">Presets</span>
                            {showPrograms && (
                                <div className="program-dropdown">
                                    {factoryPrograms.map((p, i) => (
                                        <div key={i} className="program-item" onClick={() => handleProgramChange(i)}>
                                            {p}
                                        </div>
                                    ))}
                                </div>
                            )}
                        </div>
                    )}
                    {/* GUI editor */}
                    {onOpenEditor && (
                        <button
                            className="btn-rack-edit"
                            onClick={() => onOpenEditor(trackIndex, effect.id, effect.name)}
                            title={isNative ? 'Open GUI Editor' : 'Open Plugin GUI'}
                        >
                            GUI
                        </button>
                    )}
                    {/* Delete */}
                    {onRemove && (
                        <button
                            className="btn-rack-remove"
                            onClick={() => onRemove(trackIndex, effect.id)}
                            title="Remove Effect"
                        >
                            ×
                        </button>
                    )}
                </div>
            </div>

            {/* Preset Browser Dropdown */}
            {showPresets && (
                <div className="rack-preset-dropdown">
                    <PluginPresetBrowser
                        trackIdx={trackIndex}
                        pluginId={effect.id}
                        pluginName={effect.name}
                        onClose={() => setShowPresets(false)}
                    />
                </div>
            )}

            {/* Parameter Knobs */}
            <div className="rack-params">
                {effect.parameters.map(param => (
                    <ParamKnob
                        key={param.id}
                        param={param}
                        onChange={onParamChange}
                        size={32}
                        color={isNative ? "#00e5ff" : "#ff00ff"}
                    />
                ))}
                {effect.parameters.length === 0 && (
                    <div className="no-params">No Automatable Parameters</div>
                )}
            </div>

            {/* Type badge */}
            {!isNative && (
                <div className="wasm-badge">EXT</div>
            )}

            {/* Right-click context menu — rendered in document body position */}
            {ctxMenu && (
                <div
                    ref={ctxRef}
                    className="rack-ctx-menu"
                    style={{ top: ctxMenu.y, left: ctxMenu.x }}
                >
                    <div className="rack-ctx-header">{effect.name}</div>
                    <button className="rack-ctx-item" onClick={() => {
                        onBypassToggle(trackIndex, effect.id, effect.is_bypassed);
                        closeCtx();
                    }}>
                        {effect.is_bypassed ? '▶ Enable' : '⏸ Bypass'}
                    </button>
                    {onOpenEditor && (
                        <button className="rack-ctx-item" onClick={() => {
                            onOpenEditor(trackIndex, effect.id, effect.name);
                            closeCtx();
                        }}>
                            🖥 Open GUI
                        </button>
                    )}
                    <button className="rack-ctx-item" onClick={() => {
                        setShowPresets(true);
                        closeCtx();
                    }}>
                        📋 Presets…
                    </button>
                    <div className="rack-ctx-divider" />
                    {onRemove && (
                        <button className="rack-ctx-item rack-ctx-danger" onClick={() => {
                            onRemove(trackIndex, effect.id);
                            closeCtx();
                        }}>
                            🗑 Delete Plugin
                        </button>
                    )}
                </div>
            )}
        </div>
    );
};
