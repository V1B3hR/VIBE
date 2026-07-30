
import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './PresetBrowser.css';

interface PresetBrowserProps {
    onLoad: (path: string) => void;
    onSave: (name: string) => void;
}

const PresetBrowser: React.FC<PresetBrowserProps> = ({ onLoad, onSave }) => {
    const [presets, setPresets] = useState<string[]>([]);
    const [newName, setNewName] = useState("");

    const loadPresets = async () => {
        try {
            const list = await invoke<string[]>("list_synth_presets");
            setPresets(list);
        } catch (e) {
            console.error("Failed to load presets", e);
        }
    };

    useEffect(() => {
        loadPresets();
    }, []);

    return (
        <div className="preset-browser glass">
            <div className="preset-header">
                <span className="browser-title">Librarian</span>
                <div className="save-row">
                    <input
                        value={newName}
                        onChange={e => setNewName(e.target.value)}
                        placeholder="Preset name..."
                        className="preset-input"
                    />
                    <button className="save-btn" onClick={() => { if (newName) { onSave(newName); loadPresets(); } }}>Save</button>
                </div>
            </div>
            <div className="preset-list">
                {presets.length === 0 && <div className="no-presets">No presets found</div>}
                {presets.map(p => (
                    <div key={p} className="preset-item" onClick={() => onLoad(`presets/synth/${p}`)}>
                        <span className="preset-name">{p.replace(".json", "").replace(".vone", "")}</span>
                        <span className="preset-ext">.vone</span>
                    </div>
                ))}
            </div>
        </div>
    );
};

export default PresetBrowser;
