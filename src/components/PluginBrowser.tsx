import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { PluginInfo, PluginCategory, PluginChain } from '../types/plugin';
import { PluginCard } from './PluginCard';
import { PluginContextMenu } from './PluginContextMenu';
import { PluginChains } from './PluginChains';
import { PluginPerformance } from './PluginPerformance';
import { PluginManagement } from './PluginManagement';
import { PluginDatabase } from './PluginDatabase';
import { PluginDiagnosticsView } from './PluginDiagnosticsView';
import './PluginBrowser.css';

interface PluginRecommendation {
    name: string;
    description: string;
    plugin_type: string;
}

const AiPluginTips = ({ activeView }: { activeView: string }) => {
    const [tips, setTips] = useState<PluginRecommendation[]>([]);

    useEffect(() => {
        // Query Kropelka Brain for recommendations based on what the user is looking at
        invoke<PluginRecommendation[]>('query_plugin_database', { category: activeView })
            .then(res => setTips(res))
            .catch(console.error);
    }, [activeView]);

    if (!tips || tips.length === 0) return null;

    return (
        <div className="ai-plugin-tips">
            <div className="ai-tips-header">
                <span className="ai-avatar-mini">💧</span> Kropelka's Studio Advice: {activeView.toUpperCase()}
            </div>
            <div className="ai-tips-list">
                {tips.map((t, idx) => (
                    <div key={idx} className="ai-tip-card">
                        <div className="ai-tip-title">{t.name} <span className="ai-tip-badge">{t.plugin_type}</span></div>
                        <div className="ai-tip-desc">{t.description}</div>
                    </div>
                ))}
            </div>
        </div>
    );
};

interface PluginBrowserProps {
    onLoadPlugin?: (plugin: PluginInfo) => void;
    onLoadChain?: (chain: PluginChain) => void;
}

interface ContextMenuState {
    x: number;
    y: number;
    plugin: PluginInfo;
}

type BrowserView = PluginCategory | 'All' | 'Favorites' | 'Recent' | 'Folders' | 'Management' | 'Performance' | 'Chains' | 'Database';

export const PluginBrowser: React.FC<PluginBrowserProps> = ({ onLoadPlugin, onLoadChain }) => {
    const [plugins, setPlugins] = useState<PluginInfo[]>([]);
    const [searchQuery, setSearchQuery] = useState('');
    const [activeView, setActiveView] = useState<BrowserView>('All');
    const [isLoading, setIsLoading] = useState(true);
    const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
    const [selectedFolder, setSelectedFolder] = useState<string | null>(null);
    const [diagnosticsPlugin, setDiagnosticsPlugin] = useState<PluginInfo | null>(null);

    const mainViews: BrowserView[] = ['All', 'Favorites', 'Recent', 'Folders', 'Chains', 'Performance', 'Management', 'Database'];
    const categoryViews: PluginCategory[] = ['Instrument', 'Dynamics', 'EQ', 'Reverb', 'Delay', 'Distortion', 'Modulation', 'Utility', 'MidiFX'];

    const fetchPlugins = useCallback(async () => {
        setIsLoading(true);
        try {
            let result: PluginInfo[] = [];

            if (activeView === 'All' || activeView === 'Folders' || activeView === 'Management' || activeView === 'Performance' || activeView === 'Database') {
                result = await invoke<PluginInfo[]>('plugin_get_all');
                if (activeView === 'Folders' && selectedFolder) {
                    result = result.filter(p => p.custom_folder === selectedFolder);
                }
            } else if (activeView === 'Favorites') {
                result = await invoke<PluginInfo[]>('plugin_get_favorites');
            } else if (activeView === 'Recent') {
                result = await invoke<PluginInfo[]>('plugin_get_recent', { limit: 20 });
            } else if (activeView === 'Chains') {
                result = [];
            } else {
                result = await invoke<PluginInfo[]>('plugin_get_by_category', { category: activeView as PluginCategory });
            }

            setPlugins(result);
        } catch (e) {
            console.error('Failed to fetch plugins:', e);
        } finally {
            setIsLoading(false);
        }
    }, [activeView, selectedFolder]);

    useEffect(() => {
        fetchPlugins();
    }, [fetchPlugins]);

    const [selectedTags, setSelectedTags] = useState<string[]>([]);

    const allTags = useMemo(() => {
        const tags = new Set<string>();
        plugins.forEach(p => p.tags.forEach(t => tags.add(t)));
        return Array.from(tags).sort();
    }, [plugins]);

    const allFolders = useMemo(() => {
        const folders = new Set<string>();
        plugins.forEach(p => p.custom_folder && folders.add(p.custom_folder));
        return Array.from(folders).sort();
    }, [plugins]);

    const filteredPlugins = useMemo(() => {
        let result = plugins;

        // Final display filtering
        if (activeView !== 'Management' && activeView !== 'Database') {
            result = result.filter(p => !p.hidden);
        }

        if (searchQuery) {
            const query = searchQuery.toLowerCase();
            result = result.filter(p =>
                p.name.toLowerCase().includes(query) ||
                p.vendor.toLowerCase().includes(query) ||
                p.tags.some(t => t.toLowerCase().includes(query)) ||
                p.custom_folder?.toLowerCase().includes(query)
            );
        }

        if (selectedTags.length > 0) {
            result = result.filter(p =>
                selectedTags.every(tag => p.tags.includes(tag))
            );
        }

        return result;
    }, [plugins, searchQuery, selectedTags, activeView]);

    const toggleTag = (tag: string) => {
        setSelectedTags(prev =>
            prev.includes(tag) ? prev.filter(t => t !== tag) : [...prev, tag]
        );
    };

    useEffect(() => {
        setSelectedTags([]);
        if (activeView !== 'Folders') setSelectedFolder(null);
    }, [activeView]);

    const handleToggleFavorite = async (pluginId: string) => {
        try {
            await invoke('plugin_toggle_favorite', { pluginId });
            fetchPlugins();
        } catch (e) {
            console.error('Toggle favorite failed:', e);
        }
    };

    const handleSelectPlugin = (plugin: PluginInfo) => {
        if (onLoadPlugin) {
            onLoadPlugin(plugin);
        }
        invoke('plugin_update_last_used', { pluginId: plugin.id }).catch(console.error);
    };

    const handleContextMenu = (e: React.MouseEvent, plugin: PluginInfo) => {
        setContextMenu({
            x: e.clientX,
            y: e.clientY,
            plugin
        });
    };

    const handleMenuAction = async (action: string, plugin: PluginInfo, payload?: any) => {
        setContextMenu(null);

        switch (action) {
            case 'load':
                handleSelectPlugin(plugin);
                break;
            case 'toggle_favorite':
                handleToggleFavorite(plugin.id);
                break;
            case 'blacklist':
                if (confirm(`Blacklist ${plugin.name}? It won't be scanned again.`)) {
                    await invoke('plugin_handle_blacklist', { pluginId: plugin.id, reason: 'Manual' });
                    fetchPlugins();
                }
                break;
            case 'set_hidden':
                await invoke('plugin_set_hidden', { pluginId: plugin.id, hidden: payload });
                fetchPlugins();
                break;
            case 'set_deprecated':
                await invoke('plugin_set_deprecated', { pluginId: plugin.id, deprecated: payload });
                fetchPlugins();
                break;
            case 'merge_duplicates':
                const otherId = prompt(`Merge ${plugin.name} with (Enter other Plugin ID):`);
                if (otherId) {
                    await invoke('plugin_merge_duplicates', { primaryId: otherId, duplicateId: plugin.id });
                    fetchPlugins();
                }
                break;
            case 'show_diagnostics':
                setDiagnosticsPlugin(plugin);
                break;
            case 'set_folder':
                const folder = prompt("Assign to Folder (Leave empty to remove):", plugin.custom_folder || "");
                if (folder !== null) {
                    await invoke('plugin_set_custom_folder', { pluginId: plugin.id, folder: folder || null });
                    fetchPlugins();
                }
                break;
            case 'rescan_single':
                await invoke('scan_plugins');
                fetchPlugins();
                break;
        }
    };

    return (
        <div className="plugin-browser">
            <div className="plugin-browser-categories">
                <div className="view-row">
                    {mainViews.map(view => (
                        <button
                            key={view}
                            className={`cat-tab main ${activeView === view ? 'active' : ''}`}
                            onClick={() => setActiveView(view)}
                        >
                            {view === 'Favorites' ? '⭐' : view === 'Recent' ? '🕒' : view === 'Folders' ? '📁' : view === 'Chains' ? '⛓️' : view === 'Performance' ? '⚡' : view === 'Management' ? '🛠️' : view === 'Database' ? '🗄️' : view.toUpperCase()}
                        </button>
                    ))}
                </div>
                <div className="view-row categories">
                    {categoryViews.map(cat => (
                        <button
                            key={cat}
                            className={`cat-tab ${activeView === cat ? 'active' : ''}`}
                            onClick={() => setActiveView(cat)}
                        >
                            {cat.toUpperCase()}
                        </button>
                    ))}
                </div>
            </div>

            <div className="plugin-browser-search">
                <input
                    type="text"
                    placeholder="Search plugins, vendors, tags..."
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    className="plugin-search-input"
                />
                {searchQuery && <button className="clear-search" onClick={() => setSearchQuery('')}>×</button>}
            </div>

            {activeView === 'Folders' && (
                <div className="plugin-tag-cloud folders">
                    {allFolders.map(f => (
                        <button
                            key={f}
                            className={`tag-bubble ${selectedFolder === f ? 'active' : ''}`}
                            onClick={() => setSelectedFolder(f)}
                        >
                            📁 {f}
                        </button>
                    ))}
                    {allFolders.length === 0 && (
                        <div className="no-folders-hint">Right-click a plugin to create/assign a folder.</div>
                    )}
                </div>
            )}

            {allTags.length > 0 && !['Folders', 'Management', 'Performance', 'Chains', 'Database'].includes(activeView) && (
                <div className="plugin-tag-cloud">
                    {allTags.map(tag => (
                        <button
                            key={tag}
                            className={`tag-bubble ${selectedTags.includes(tag) ? 'active' : ''}`}
                            onClick={() => toggleTag(tag)}
                        >
                            {tag}
                        </button>
                    ))}
                    {selectedTags.length > 0 && (
                        <button className="tag-bubble clear" onClick={() => setSelectedTags([])}>
                            Clear Filters
                        </button>
                    )}
                </div>
            )}

            <div className="plugin-browser-main-content">
                {isLoading ? (
                    <div className="plugin-browser-status">Updating...</div>
                ) : activeView === 'Chains' ? (
                    <PluginChains onLoadChain={onLoadChain} />
                ) : activeView === 'Performance' ? (
                    <PluginPerformance plugins={plugins} />
                ) : activeView === 'Management' ? (
                    <PluginManagement plugins={plugins} onRefresh={fetchPlugins} />
                ) : activeView === 'Database' ? (
                    <PluginDatabase />
                ) : (
                    <>
                        {['EQ', 'Compression', 'Synth', 'Instrument', 'Dynamics'].includes(activeView) && (
                            <AiPluginTips activeView={activeView === 'Dynamics' ? 'Compression' : activeView} />
                        )}
                        <div className="plugin-browser-list">
                            {filteredPlugins.length > 0 ? (
                                filteredPlugins.map(plugin => (
                                    <PluginCard
                                        key={plugin.id}
                                        plugin={plugin}
                                        onSelect={handleSelectPlugin}
                                        onToggleFavorite={handleToggleFavorite}
                                        onContextMenu={handleContextMenu}
                                    />
                                ))
                            ) : (
                                <div className="plugin-browser-status">No plugins matched.</div>
                            )}
                        </div>
                    </>
                )}
            </div>

            {contextMenu && (
                <PluginContextMenu
                    x={contextMenu.x}
                    y={contextMenu.y}
                    plugin={contextMenu.plugin}
                    onClose={() => setContextMenu(null)}
                    onAction={handleMenuAction}
                />
            )}

            {diagnosticsPlugin && (
                <PluginDiagnosticsView
                    plugin={diagnosticsPlugin}
                    onClose={() => setDiagnosticsPlugin(null)}
                />
            )}

            <div className="plugin-browser-footer">
                <div className="scan-info">
                    {plugins.length} Plugins Found
                </div>
                <button className="rescan-btn" onClick={() => invoke('scan_plugins').then(fetchPlugins)}>
                    🔄 Rescan
                </button>
            </div>
        </div>
    );
};
