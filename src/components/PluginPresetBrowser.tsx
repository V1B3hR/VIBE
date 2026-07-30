import React, { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './PluginPresetBrowser.css';

interface PluginPresetBrowserProps {
    trackIdx: number;
    pluginId: string;
    pluginName: string;
    onClose: () => void;
}

export const PluginPresetBrowser: React.FC<PluginPresetBrowserProps> = ({
    trackIdx,
    pluginId,
    pluginName,
    onClose,
}) => {
    const [presets, setPresets] = useState<string[]>([]);
    const [latency, setLatency] = useState<number | null>(null);
    const [savePresetName, setSavePresetName] = useState('');
    const [status, setStatus] = useState<string | null>(null);
    const [loading, setLoading] = useState(false);
    const [search, setSearch] = useState('');

    const fetchPresets = useCallback(async () => {
        try {
            const list = await invoke<string[]>('list_plugin_presets', { pluginName });
            setPresets(list);
        } catch (e) {
            console.error('Failed to list plugin presets:', e);
        }
    }, [pluginName]);

    const fetchLatency = useCallback(async () => {
        try {
            const lat = await invoke<number>('get_plugin_latency', { trackIdx, pluginId });
            setLatency(lat);
        } catch (_) {
            setLatency(null);
        }
    }, [trackIdx, pluginId]);

    useEffect(() => {
        fetchPresets();
        fetchLatency();
    }, [fetchPresets, fetchLatency]);

    const handleLoad = async (presetPath: string) => {
        setLoading(true);
        setStatus(null);
        try {
            await invoke('load_plugin_preset', { trackIdx, pluginId, presetPath });
            setStatus('✅ Preset loaded');
        } catch (e: any) {
            setStatus(`❌ ${e}`);
        } finally {
            setLoading(false);
        }
    };

    const handleSave = async () => {
        if (!savePresetName.trim()) return;
        setLoading(true);
        setStatus(null);
        try {
            const path = await invoke<string>('save_plugin_preset', {
                trackIdx,
                pluginId,
                presetName: savePresetName.trim(),
            });
            setStatus(`✅ Saved: ${path.split(/[\\/]/).pop()}`);
            setSavePresetName('');
            await fetchPresets();
        } catch (e: any) {
            setStatus(`❌ ${e}`);
        } finally {
            setLoading(false);
        }
    };

    const handleExportState = async () => {
        try {
            const b64 = await invoke<string>('get_plugin_state', { trackIdx, pluginId });
            if (!b64) { setStatus('ℹ️ No state to export'); return; }
            const blob = new Blob([b64], { type: 'text/plain' });
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = `${pluginName.replace(/[^a-z0-9]/gi, '_')}_state.b64`;
            a.click();
            URL.revokeObjectURL(url);
            setStatus('✅ State exported');
        } catch (e: any) {
            setStatus(`❌ ${e}`);
        }
    };

    const filteredPresets = presets.filter(p =>
        p.toLowerCase().includes(search.toLowerCase())
    );

    const presetLabel = (p: string) =>
        p.split(/[\\/]/).pop()?.replace('.vst3preset', '') ?? p;

    return (
        <div className="plugin-preset-browser" onClick={e => e.stopPropagation()}>
            <div className="ppb-header">
                <div className="ppb-title">
                    <span className="ppb-icon">🎛️</span>
                    <div>
                        <div className="ppb-name">{pluginName}</div>
                        {latency !== null && latency > 0 && (
                            <div className="ppb-latency">PDC Latency: {latency} spl</div>
                        )}
                    </div>
                </div>
                <button className="ppb-close" onClick={onClose}>×</button>
            </div>

            <div className="ppb-body">
                {/* Save Section */}
                <div className="ppb-section">
                    <div className="ppb-section-title">💾 Save Preset</div>
                    <div className="ppb-save-row">
                        <input
                            className="ppb-input"
                            placeholder="Preset name..."
                            value={savePresetName}
                            onChange={e => setSavePresetName(e.target.value)}
                            onKeyDown={e => e.key === 'Enter' && handleSave()}
                        />
                        <button
                            className="ppb-btn ppb-btn-save"
                            onClick={handleSave}
                            disabled={!savePresetName.trim() || loading}
                        >
                            Save
                        </button>
                    </div>
                </div>

                {/* Preset List */}
                <div className="ppb-section ppb-presets-section">
                    <div className="ppb-section-title">📦 Presets ({filteredPresets.length})</div>
                    {presets.length > 5 && (
                        <input
                            className="ppb-search"
                            placeholder="Search presets..."
                            value={search}
                            onChange={e => setSearch(e.target.value)}
                        />
                    )}
                    <div className="ppb-preset-list">
                        {filteredPresets.length === 0 ? (
                            <div className="ppb-empty">
                                {presets.length === 0
                                    ? 'No presets saved yet. Save your first preset above.'
                                    : 'No presets match search.'}
                            </div>
                        ) : (
                            filteredPresets.map(p => (
                                <button
                                    key={p}
                                    className="ppb-preset-item"
                                    onClick={() => handleLoad(p)}
                                    disabled={loading}
                                    title={p}
                                >
                                    <span className="ppb-preset-icon">▶</span>
                                    <span className="ppb-preset-name">{presetLabel(p)}</span>
                                </button>
                            ))
                        )}
                    </div>
                </div>

                {/* Tools Section */}
                <div className="ppb-section ppb-tools">
                    <button className="ppb-btn ppb-btn-ghost" onClick={handleExportState}>
                        📤 Export State
                    </button>
                    <button className="ppb-btn ppb-btn-ghost" onClick={fetchPresets}>
                        🔄 Refresh
                    </button>
                </div>

                {status && (
                    <div className={`ppb-status ${status.startsWith('❌') ? 'error' : 'ok'}`}>
                        {status}
                    </div>
                )}
            </div>
        </div>
    );
};
