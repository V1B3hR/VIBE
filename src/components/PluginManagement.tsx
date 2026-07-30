import React from 'react';
import { invoke } from '@tauri-apps/api/core';
import { PluginInfo } from '../types/plugin';
import './PluginManagement.css';

interface PluginManagementProps {
    plugins: PluginInfo[];
    onRefresh: () => void;
}

export const PluginManagement: React.FC<PluginManagementProps> = ({ plugins, onRefresh }) => {
    const hiddenCount = plugins.filter(p => p.hidden).length;
    const deprecatedCount = plugins.filter(p => p.deprecated).length;
    const blacklistedCount = plugins.filter(p => p.is_blacklisted).length;

    const handleBatchUnhide = async () => {
        if (confirm('Unhide all hidden plugins?')) {
            try {
                const hidden = plugins.filter(p => p.hidden);
                for (const p of hidden) {
                    await invoke('plugin_set_hidden', { pluginId: p.id, hidden: false });
                }
                onRefresh();
            } catch (e) {
                console.error('Batch unhide failed:', e);
            }
        }
    };

    const handleBatchDeprecate = async () => {
        // Example batch op: mark all VST2 as deprecated
        if (confirm('Mark all VST2 plugins as LEGACY?')) {
            try {
                const vst2 = plugins.filter(p => p.plugin_type === 'VST2');
                for (const p of vst2) {
                    await invoke('plugin_set_deprecated', { pluginId: p.id, deprecated: true });
                }
                onRefresh();
            } catch (e) {
                console.error('Batch deprecate failed:', e);
            }
        }
    };

    return (
        <div className="plugin-management">
            <div className="mgmt-stats">
                <div className="stat-card">
                    <span className="stat-label">Hidden</span>
                    <span className="stat-value">{hiddenCount}</span>
                </div>
                <div className="stat-card">
                    <span className="stat-label">Legacy</span>
                    <span className="stat-value">{deprecatedCount}</span>
                </div>
                <div className="stat-card danger">
                    <span className="stat-label">Crashed</span>
                    <span className="stat-value">{blacklistedCount}</span>
                </div>
            </div>

            <div className="mgmt-actions">
                <h4>Batch Operations</h4>
                <div className="action-grid">
                    <button className="mgmt-btn" onClick={handleBatchUnhide}>
                        🔓 Unhide All
                    </button>
                    <button className="mgmt-btn warning" onClick={handleBatchDeprecate}>
                        ⚠️ Deprecate VST2
                    </button>
                    <button className="mgmt-btn danger" onClick={() => invoke('scan_plugins').then(onRefresh)}>
                        🧹 Reset & Rescan
                    </button>
                </div>
            </div>

            <div className="mgmt-info">
                <h4>Management Mode</h4>
                <p>Right-click plugins in the browser to hide, mark as legacy, or merge duplicates.</p>
            </div>
        </div>
    );
};
