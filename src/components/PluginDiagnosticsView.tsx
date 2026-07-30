import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { PluginInfo, PluginDiagnostics, PluginPreset } from '../types/plugin';
import './PluginDiagnosticsView.css';

interface PluginDiagnosticsViewProps {
    plugin: PluginInfo;
    onClose: () => void;
}

export const PluginDiagnosticsView: React.FC<PluginDiagnosticsViewProps> = ({ plugin, onClose }) => {
    const [diagnostics, setDiagnostics] = useState<PluginDiagnostics | null>(null);
    const [presets, setPresets] = useState<PluginPreset[]>([]);
    const [isLoading, setIsLoading] = useState(true);

    useEffect(() => {
        const fetchData = async () => {
            try {
                const [diagResult, presetResult] = await Promise.all([
                    invoke<PluginDiagnostics>('plugin_get_diagnostics', { pluginId: plugin.id }),
                    invoke<PluginPreset[]>('plugin_get_presets', { pluginId: plugin.id })
                ]);
                setDiagnostics(diagResult);
                setPresets(presetResult);
            } catch (e) {
                console.error('Failed to fetch data:', e);
            } finally {
                setIsLoading(false);
            }
        };
        fetchData();
    }, [plugin.id]);

    const handleMigrate = async () => {
        if (confirm(`Attempt to migrate ${plugin.name} to VST3?`)) {
            try {
                const newId = await invoke<string>('plugin_migrate_vst2_to_vst3', { pluginId: plugin.id });
                alert(`Successfully migrated! New ID: ${newId}`);
                onClose();
            } catch (e) {
                alert(`Migration failed: ${e}`);
            }
        }
    };

    return (
        <div className="plugin-diagnostics-view glass">
            <div className="diag-header">
                <h3>Plugin Diagnostics</h3>
                <button className="close-btn" onClick={onClose}>×</button>
            </div>

            <div className="diag-plugin-summary">
                <div className="diag-icon">{plugin.category === 'Instrument' ? '🎹' : '🎚️'}</div>
                <div className="diag-titles">
                    <span className="diag-name">{plugin.name}</span>
                    <span className="diag-vendor">{plugin.vendor} • {plugin.plugin_type}</span>
                </div>
            </div>

            <div className="diag-content">
                {isLoading ? (
                    <div className="diag-status">Analyzing plugin internal state...</div>
                ) : diagnostics ? (
                    <div className="diag-grid">
                        <div className="diag-stat">
                            <span className="label">Load Time</span>
                            <span className="value">{diagnostics.load_time_ms} ms</span>
                        </div>
                        <div className="diag-stat">
                            <span className="label">Memory Usage</span>
                            <span className="value">{(diagnostics.memory_usage_bytes / (1024 * 1024)).toFixed(1)} MB</span>
                        </div>
                        <div className="diag-stat">
                            <span className="label">Error Count</span>
                            <span className="value">{diagnostics.error_count}</span>
                        </div>
                        <div className="diag-stat">
                            <span className="label">Last Crash</span>
                            <span className="value">{diagnostics.last_crash ? new Date(diagnostics.last_crash * 1000).toLocaleString() : 'Never'}</span>
                        </div>
                    </div>
                ) : (
                    <div className="diag-error">Failed to retrieve diagnostics data.</div>
                )}

                <div className="diag-presets-section">
                    <h4>Parameters Preview</h4>
                    <div className="diag-parameter-grid">
                        {plugin.category === 'Instrument' ? (
                            <>
                                <div className="diag-param-preview"><span>Cutoff</span><div className="preview-bar"><div style={{ width: '75%' }} /></div></div>
                                <div className="diag-param-preview"><span>Resonance</span><div className="preview-bar"><div style={{ width: '30%' }} /></div></div>
                                <div className="diag-param-preview"><span>Attack</span><div className="preview-bar"><div style={{ width: '5%' }} /></div></div>
                                <div className="diag-param-preview"><span>Decay</span><div className="preview-bar"><div style={{ width: '45%' }} /></div></div>
                            </>
                        ) : plugin.category === 'Dynamics' ? (
                            <>
                                <div className="diag-param-preview"><span>Threshold</span><div className="preview-bar"><div style={{ width: '60%' }} /></div></div>
                                <div className="diag-param-preview"><span>Ratio</span><div className="preview-bar"><div style={{ width: '40%' }} /></div></div>
                                <div className="diag-param-preview"><span>Attack</span><div className="preview-bar"><div style={{ width: '10%' }} /></div></div>
                                <div className="diag-param-preview"><span>Release</span><div className="preview-bar"><div style={{ width: '35%' }} /></div></div>
                            </>
                        ) : (
                            <>
                                <div className="diag-param-preview"><span>Mix / Wet</span><div className="preview-bar"><div style={{ width: '50%' }} /></div></div>
                                <div className="diag-param-preview"><span>Gain</span><div className="preview-bar"><div style={{ width: '70%' }} /></div></div>
                            </>
                        )}
                    </div>
                </div>

                <div className="diag-presets-section">
                    <h4>Available Presets</h4>
                    <div className="diag-preset-list">
                        {presets.length > 0 ? presets.map((p, i) => (
                            <div key={i} className="diag-preset-item">
                                <span className="preset-name">{p.name}</span>
                                <span className="preset-cat">{p.category}</span>
                            </div>
                        )) : (
                            <div className="diag-no-presets">No presets found for this plugin.</div>
                        )}
                    </div>
                </div>
            </div>

            <div className="diag-actions">
                {plugin.plugin_type === 'VST2' && (
                    <button className="diag-btn migrate" onClick={handleMigrate}>
                        🚀 Upgrade to VST3
                    </button>
                )}
                <button className="diag-btn" onClick={() => invoke('plugin_handle_blacklist', { pluginId: plugin.id, reason: 'Technical' }).then(onClose)}>
                    🚫 Force Blacklist
                </button>
            </div>
        </div>
    );
};
