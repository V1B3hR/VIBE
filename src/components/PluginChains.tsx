import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { PluginChain } from '../types/plugin';
import './PluginChains.css';

interface PluginChainsProps {
    onLoadChain?: (chain: PluginChain) => void;
}

export const PluginChains: React.FC<PluginChainsProps> = ({ onLoadChain }) => {
    const [chains, setChains] = useState<PluginChain[]>([]);
    const [isLoading, setIsLoading] = useState(true);

    const fetchChains = async () => {
        setIsLoading(true);
        try {
            const result = await invoke<PluginChain[]>('plugin_get_all_chains');
            setChains(result);
        } catch (e) {
            console.error('Failed to fetch chains:', e);
        } finally {
            setIsLoading(false);
        }
    };

    useEffect(() => {
        fetchChains();
    }, []);

    const handleDeleteChain = async (id: string) => {
        if (confirm('Delete this FX Chain?')) {
            try {
                await invoke('plugin_delete_chain', { chainId: id });
                fetchChains();
            } catch (e) {
                console.error('Delete failed:', e);
            }
        }
    };

    const handleLoadChain = (chain: PluginChain) => {
        if (onLoadChain) {
            onLoadChain(chain);
        }
    };

    return (
        <div className="plugin-chains">
            <div className="chains-header">
                <h3>FX Chains</h3>
                <button className="refresh-btn" onClick={fetchChains}>🔄</button>
            </div>

            {isLoading ? (
                <div className="chains-status">Loading chains...</div>
            ) : chains.length > 0 ? (
                <div className="chains-list">
                    {chains.map(chain => (
                        <div key={chain.id} className="chain-item" onClick={() => handleLoadChain(chain)}>
                            <div className="chain-info">
                                <span className="chain-name">{chain.name}</span>
                                <span className="chain-meta">
                                    {chain.plugins.length} Plugins • {chain.routing}
                                </span>
                            </div>
                            <button
                                className="delete-chain-btn"
                                onClick={(e) => {
                                    e.stopPropagation();
                                    handleDeleteChain(chain.id);
                                }}
                            >
                                ×
                            </button>
                        </div>
                    ))}
                </div>
            ) : (
                <div className="chains-empty">
                    <p>No saved FX chains.</p>
                    <small>Save a track's FX chain to see it here.</small>
                </div>
            )}
        </div>
    );
};
