import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './IoSettings.css';

interface InputAlias {
    id: string;
    name: string;
    is_stereo: boolean;
    hardware_channels: number[];
    color: string;
}

interface IoSettingsProps {
    onClose: () => void;
}

const IoSettings: React.FC<IoSettingsProps> = ({ onClose }) => {
    const [aliases, setAliases] = useState<InputAlias[]>([]);
    const [channelMeters, setChannelMeters] = useState<number[]>(new Array(64).fill(0));
    const [selectedAlias, setSelectedAlias] = useState<string | null>(null);
    const [editingAlias, setEditingAlias] = useState<string | null>(null);
    const [editName, setEditName] = useState('');

    // Load aliases on mount
    useEffect(() => {
        loadAliases();
    }, []);

    // Poll channel meters at 60 FPS
    useEffect(() => {
        const interval = setInterval(async () => {
            try {
                const meters = await invoke<number[]>('get_channel_meters');
                setChannelMeters(meters);
            } catch (error) {
                console.error('Failed to get channel meters:', error);
            }
        }, 16); // ~60 FPS

        return () => clearInterval(interval);
    }, []);

    const loadAliases = async () => {
        try {
            const result = await invoke<InputAlias[]>('get_input_aliases');
            setAliases(result);
        } catch (error) {
            console.error('Failed to load aliases:', error);
        }
    };

    const createAlias = async () => {
        try {
            const id = await invoke<string>('create_input_alias', {
                name: 'New Input',
                isStereo: false,
                channels: [0],
                color: '#FFD700',
            });
            await loadAliases();
            setSelectedAlias(id);
        } catch (error) {
            console.error('Failed to create alias:', error);
        }
    };

    const updateAlias = async (id: string, name: string, channels: number[], color: string) => {
        try {
            await invoke('update_input_alias', { id, name, channels, color });
            await loadAliases();
        } catch (error) {
            console.error('Failed to update alias:', error);
        }
    };

    const deleteAlias = async (id: string) => {
        try {
            await invoke('delete_input_alias', { id });
            await loadAliases();
            if (selectedAlias === id) {
                setSelectedAlias(null);
            }
        } catch (error) {
            console.error('Failed to delete alias:', error);
        }
    };

    const handleChannelClick = (channelIndex: number) => {
        if (!selectedAlias) return;

        const alias = aliases.find(a => a.id === selectedAlias);
        if (!alias) return;

        const currentChannels = alias.hardware_channels;
        const isAssigned = currentChannels.includes(channelIndex);

        if (isAssigned) {
            // Unassign
            const newChannels = currentChannels.filter(ch => ch !== channelIndex);
            if (newChannels.length > 0) {
                updateAlias(alias.id, alias.name, newChannels, alias.color);
            }
        } else {
            // Assign
            if (alias.is_stereo) {
                // For stereo, replace both channels
                updateAlias(alias.id, alias.name, [channelIndex, channelIndex + 1], alias.color);
            } else {
                // For mono, replace single channel
                updateAlias(alias.id, alias.name, [channelIndex], alias.color);
            }
        }
    };

    const startEdit = (alias: InputAlias) => {
        setEditingAlias(alias.id);
        setEditName(alias.name);
    };

    const finishEdit = (alias: InputAlias) => {
        if (editName.trim()) {
            updateAlias(alias.id, editName.trim(), alias.hardware_channels, alias.color);
        }
        setEditingAlias(null);
    };

    const getChannelIntensity = (meter: number): number => {
        // Convert RMS to visual intensity (0-1)
        // RMS is typically 0.0 to 1.0, but we want to emphasize lower values
        return Math.min(1.0, meter * 2.0);
    };

    const isChannelAssigned = (channelIndex: number): InputAlias | null => {
        return aliases.find(alias => alias.hardware_channels.includes(channelIndex)) || null;
    };

    return (
        <div className="io-settings-overlay" onClick={onClose}>
            <div className="io-settings-modal" onClick={(e) => e.stopPropagation()}>
                <div className="io-settings-header">
                    <h2>⚙️ Hardware I/O Settings</h2>
                    <button className="close-btn" onClick={onClose}>✕</button>
                </div>

                <div className="io-settings-content">
                    {/* Left Panel: Alias List */}
                    <div className="alias-panel">
                        <div className="alias-panel-header">
                            <h3>Input Aliases</h3>
                            <button className="btn-add-alias" onClick={createAlias}>
                                + New Alias
                            </button>
                        </div>

                        <div className="alias-list">
                            {aliases.map(alias => (
                                <div
                                    key={alias.id}
                                    className={`alias-item ${selectedAlias === alias.id ? 'selected' : ''}`}
                                    onClick={() => setSelectedAlias(alias.id)}
                                    style={{ borderLeftColor: alias.color }}
                                >
                                    <div className="alias-icon">
                                        {alias.is_stereo ? '🎹' : '🎤'}
                                    </div>
                                    <div className="alias-info">
                                        {editingAlias === alias.id ? (
                                            <input
                                                type="text"
                                                value={editName}
                                                onChange={(e) => setEditName(e.target.value)}
                                                onBlur={() => finishEdit(alias)}
                                                onKeyDown={(e) => {
                                                    if (e.key === 'Enter') finishEdit(alias);
                                                    if (e.key === 'Escape') setEditingAlias(null);
                                                }}
                                                autoFocus
                                                className="alias-name-input"
                                            />
                                        ) : (
                                            <div className="alias-name" onDoubleClick={() => startEdit(alias)}>
                                                {alias.name}
                                            </div>
                                        )}
                                        <div className="alias-channels">
                                            Ch: {alias.hardware_channels.join(', ')}
                                        </div>
                                    </div>
                                    <div className="alias-actions">
                                        <button
                                            className="btn-icon"
                                            onClick={(e) => {
                                                e.stopPropagation();
                                                startEdit(alias);
                                            }}
                                            title="Edit"
                                        >
                                            ✏️
                                        </button>
                                        <button
                                            className="btn-icon"
                                            onClick={(e) => {
                                                e.stopPropagation();
                                                deleteAlias(alias.id);
                                            }}
                                            title="Delete"
                                        >
                                            🗑️
                                        </button>
                                    </div>
                                </div>
                            ))}

                            {aliases.length === 0 && (
                                <div className="empty-state">
                                    <p>No input aliases yet.</p>
                                    <p>Click "+ New Alias" to create one.</p>
                                </div>
                            )}
                        </div>
                    </div>

                    {/* Right Panel: Channel Grid */}
                    <div className="channel-panel">
                        <div className="channel-panel-header">
                            <h3>Hardware Channels (1-64)</h3>
                            {selectedAlias && (
                                <div className="selected-alias-hint">
                                    Selected: {aliases.find(a => a.id === selectedAlias)?.name}
                                </div>
                            )}
                        </div>

                        <div className="channel-grid">
                            {channelMeters.map((meter, index) => {
                                const assignedAlias = isChannelAssigned(index);
                                const intensity = getChannelIntensity(meter);

                                return (
                                    <div
                                        key={index}
                                        className={`channel-cell ${assignedAlias ? 'assigned' : ''} ${selectedAlias && assignedAlias?.id === selectedAlias ? 'selected-alias' : ''
                                            }`}
                                        onClick={() => handleChannelClick(index)}
                                        style={{
                                            backgroundColor: assignedAlias
                                                ? assignedAlias.color + '40' // 25% opacity
                                                : undefined,
                                            boxShadow: intensity > 0.1
                                                ? `inset 0 0 ${10 + intensity * 20}px rgba(0, 255, 0, ${intensity})`
                                                : undefined,
                                        }}
                                        title={assignedAlias ? `${assignedAlias.name} (Ch ${index + 1})` : `Channel ${index + 1}`}
                                    >
                                        <div className="channel-number">{index + 1}</div>
                                        {assignedAlias && (
                                            <div className="channel-assigned-marker">✓</div>
                                        )}
                                    </div>
                                );
                            })}
                        </div>

                        <div className="channel-legend">
                            <div className="legend-item">
                                <div className="legend-box empty"></div>
                                <span>Unassigned</span>
                            </div>
                            <div className="legend-item">
                                <div className="legend-box assigned"></div>
                                <span>Assigned</span>
                            </div>
                            <div className="legend-item">
                                <div className="legend-box pulsing"></div>
                                <span>Signal Active</span>
                            </div>
                        </div>
                    </div>
                </div>

                <div className="io-settings-footer">
                    <div className="footer-info">
                        <span>💡 Tip: Click a channel to assign it to the selected alias</span>
                    </div>
                    <button className="btn-primary" onClick={onClose}>
                        Done
                    </button>
                </div>
            </div>
        </div>
    );
};

export default IoSettings;
