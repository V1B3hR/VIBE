import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ScanLogEntry } from '../types/plugin';
import './PluginDatabase.css';

export const PluginDatabase: React.FC = () => {
    const [paths, setPaths] = useState<string[]>([]);
    const [newPath, setNewPath] = useState('');
    const [scanLog, setScanLog] = useState<ScanLogEntry[]>([]);
    const [isScanning, setIsScanning] = useState(false);

    const fetchData = async () => {
        try {
            const currentPaths = await invoke<string[]>('plugin_get_search_paths');
            const log = await invoke<ScanLogEntry[]>('plugin_get_scan_log');
            setPaths(currentPaths);
            setScanLog(log);
        } catch (e) {
            console.error('Failed to fetch database data:', e);
        }
    };

    useEffect(() => {
        fetchData();
        const interval = setInterval(fetchData, 5000); // Auto-refresh log
        return () => clearInterval(interval);
    }, []);

    const handleAddPath = async () => {
        if (!newPath) return;
        try {
            await invoke('plugin_add_search_path', { path: newPath });
            setNewPath('');
            fetchData();
        } catch (e) {
            console.error('Failed to add path:', e);
        }
    };

    const handleRemovePath = async (path: string) => {
        try {
            await invoke('plugin_remove_search_path', { path });
            fetchData();
        } catch (e) {
            console.error('Failed to remove path:', e);
        }
    };

    const handleRescan = async () => {
        setIsScanning(true);
        try {
            await invoke('scan_plugins');
            fetchData();
        } finally {
            setIsScanning(false);
        }
    };

    return (
        <div className="plugin-database">
            <section className="db-section">
                <h4>Search Directories</h4>
                <div className="path-manager">
                    <div className="path-list">
                        {paths.map(p => (
                            <div key={p} className="path-item">
                                <span className="path-text" title={p}>{p}</span>
                                <button className="remove-path-btn" onClick={() => handleRemovePath(p)}>×</button>
                            </div>
                        ))}
                        {paths.length === 0 && <div className="empty-hint">No custom paths added.</div>}
                    </div>
                    <div className="add-path-row">
                        <input
                            type="text"
                            placeholder="C:\Program Files\Common Files\VST3..."
                            value={newPath}
                            onChange={(e) => setNewPath(e.target.value)}
                        />
                        <button className="add-btn" onClick={handleAddPath}>+ Add Path</button>
                    </div>
                </div>
            </section>

            <section className="db-section log-section">
                <div className="section-header">
                    <h4>Scan Log</h4>
                    <button className={`rescan-main-btn ${isScanning ? 'spinning' : ''}`} onClick={handleRescan}>
                        {isScanning ? '⌛ Scanning...' : '🔄 Rescan All'}
                    </button>
                </div>
                <div className="scan-log-viewer">
                    {scanLog.length > 0 ? (
                        <table className="scan-log-table">
                            <thead>
                                <tr>
                                    <th>Status</th>
                                    <th>Path</th>
                                    <th>Error</th>
                                </tr>
                            </thead>
                            <tbody>
                                {scanLog.slice().reverse().map((entry, i) => (
                                    <tr key={i} className={`log-row ${entry.status.toLowerCase()}`}>
                                        <td className="status-cell">
                                            <span className="status-icon">
                                                {entry.status === 'Success' ? '✅' : entry.status === 'Failed' ? '❌' : '⚠️'}
                                            </span>
                                            {entry.status}
                                        </td>
                                        <td className="path-cell" title={entry.path}>{entry.path}</td>
                                        <td className="error-cell">{entry.error || '-'}</td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    ) : (
                        <div className="empty-log">Log is empty. Start a scan to see details.</div>
                    )}
                </div>
            </section>
        </div>
    );
};
