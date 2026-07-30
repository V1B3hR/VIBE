import React, { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import "./Library.css";
import { LibraryItem, LibraryItemData } from "./LibraryItem";
import { PluginBrowser } from "./PluginBrowser";
import { GeneratorsPanel } from "./GeneratorsPanel";
import { PluginInfo } from "../types/plugin";

const VirtualScroll = ({
    items,
    renderItem,
    itemHeight = 90
}: {
    items: any[],
    renderItem: (item: any, style: React.CSSProperties) => React.ReactNode,
    itemHeight?: number
}) => {
    const [scrollTop, setScrollTop] = useState(0);
    const containerRef = useRef<HTMLDivElement>(null);
    const [containerHeight, setContainerHeight] = useState(600);

    useEffect(() => {
        const resizeObserver = new ResizeObserver((entries) => {
            for (let entry of entries) {
                setContainerHeight(entry.contentRect.height);
            }
        });
        if (containerRef.current) resizeObserver.observe(containerRef.current);
        return () => resizeObserver.disconnect();
    }, []);

    const totalHeight = items.length * itemHeight;
    const startIndex = Math.max(0, Math.floor(scrollTop / itemHeight) - 2);
    const visibleCount = Math.ceil(containerHeight / itemHeight) + 4;
    const endIndex = Math.min(items.length, startIndex + visibleCount);

    const visibleItems = [];
    for (let i = startIndex; i < endIndex; i++) {
        const item = items[i];
        visibleItems.push(renderItem(item, {
            position: 'absolute',
            top: i * itemHeight,
            left: 0,
            width: '100%',
            height: itemHeight - 8
        }));
    }

    return (
        <div
            ref={containerRef}
            className="library-items"
            style={{ position: 'relative', overflowY: 'auto', flex: 1 }}
            onScroll={(e) => setScrollTop(e.currentTarget.scrollTop)}
        >
            <div style={{ height: totalHeight, position: 'relative' }}>
                {visibleItems}
            </div>
        </div>
    );
};

export const Library = () => {
    const [items, setItems] = useState<LibraryItemData[]>([]);
    const [plugins, setPlugins] = useState<LibraryItemData[]>([]);
    const [tab, setTab] = useState<'samples' | 'plugins' | 'native' | 'generators'>('samples');
    const [syncPreview, setSyncPreview] = useState(true);
    const [quantizeStrength, setQuantizeStrength] = useState(1.0);
    const [quantizeDivision, setQuantizeDivision] = useState('1Bar');
    const [swing, setSwing] = useState(0.0);

    const fetchData = useCallback(async () => {
        try {
            // Need to map backend format to frontend format
            const lib = await invoke<any[]>("get_library");
            const formattedLib: LibraryItemData[] = lib.map(i => ({
                id: i.id,
                name: i.name,
                path: i.path,
                category: i.category,
                duration_samples: i.duration_seconds * 44100, // Estimate for display if needed
                waveform_peaks: i.peaks // Map backend peaks
            }));
            setItems(formattedLib);

            const pl = await invoke<any[]>("get_plugins");
            const formattedPlugins: LibraryItemData[] = pl.map(p => ({
                id: p.id,
                name: p.name,
                path: p.path,
                category: "Plugin"
            }));
            setPlugins(formattedPlugins);
        } catch (e) {
            console.error("Library fetch failed:", e);
        }
    }, []);

    useEffect(() => {
        fetchData();

        // Listen for library updates (filesystem watcher)
        let unlisten: UnlistenFn;
        const setupListener = async () => {
            unlisten = await listen('library-update', () => {
                console.log("⚡ Library Update Event Received");
                fetchData();
            });
        };
        setupListener();

        return () => {
            if (unlisten) unlisten();
        };
    }, [fetchData]);

    const handleImport = async () => {
        try {
            const selected = await open({
                multiple: false,
                filters: [{
                    name: 'Audio Files',
                    extensions: ['wav', 'mp3', 'flac', 'ogg', 'aiff']
                }]
            });

            if (selected && typeof selected === 'string') {
                await invoke("import_to_library", { path: selected });
                fetchData();
            }
        } catch (e) { console.error(e); }
    };

    const handleAddFolder = async () => {
        try {
            const selected = await open({
                directory: true,
                multiple: false
            });
            if (selected && typeof selected === 'string') {
                await invoke("library_add_directory", { path: selected });
                fetchData();
            }
        } catch (e) {
            console.error("Add folder failed:", e);
        }
    };

    const handleScanPlugins = async () => {
        await invoke("scan_plugins");
        fetchData();
    };

    const handleImportPlugin = async () => {
        try {
            const selected = await open({
                multiple: false,
                filters: [{
                    name: 'Plugins',
                    extensions: ['dll', 'vst3', 'wasm', 'vst']
                }]
            });

            if (selected && typeof selected === 'string') {
                await invoke("import_plugin", { path: selected });
                fetchData();
            }
        } catch (e) {
            console.error("Import plugin failed:", e);
        }
    };

    // Quick Actions
    const handleCreateAudioTrack = async () => {
        try { await invoke("create_audio_track", { name: "Audio Track" }); }
        catch (e) { console.error(e); }
    };


    const handleAddGroup = async () => {
        try { await invoke("create_track_group", { name: "Group" }); }
        catch (e) { console.error(e); }
    };

    // Preview Logic
    const handlePreviewStart = async (path: string) => {
        if (!path) return;
        try {
            // Call the new beat-aligned preview
            await invoke("preview_sample_synced", {
                path,
                quantize: syncPreview ? quantizeDivision : null,
                stretch: syncPreview,
                strength: quantizeStrength,
                swing: swing
            });
        } catch (e) {
            console.error("Preview failed:", e);
        }
    };

    const handlePreviewStop = async () => {
        try {
            await invoke("stop_preview");
        } catch (e) { console.error(e); }
    };

    const handleLoadPlugin = async (plugin: PluginInfo) => {
        try {
            // Check if it's a native, WASM or VST
            if (plugin.plugin_type === 'Native') {
                await invoke("add_effect", { index: 0, effectType: plugin.id });
            } else if (plugin.plugin_type === 'WASM') {
                await invoke("add_wasm_plugin", { track_idx: 0, path: plugin.path });
            } else {
                // For now, load into the first available track or active track
                await invoke("add_plugin_to_track", { trackIndex: 0, pluginPath: plugin.path });
            }
        } catch (e) {
            console.error("Load plugin failed:", e);
        }
    };

    return (
        <div className="library-panel">
            {/* Quick Access */}
            <div className="library-quick-access">
                <div className="quick-access-row">
                    <button className="quick-action-btn" onClick={handleCreateAudioTrack} title="New Empty Track">+ Track</button>
                    <button className="quick-action-btn" onClick={handleImport} title="Import Audio File">+ Import</button>
                    <button className="quick-action-btn" onClick={handleAddGroup} title="New Group">+ Group</button>
                </div>

                <div className="sync-controls-expanded">
                    <button
                        className={`sync-toggle-btn ${syncPreview ? 'active' : ''}`}
                        onClick={() => setSyncPreview(!syncPreview)}
                        title="Sync previews to Project BPM"
                    >
                        {syncPreview ? '🔄 Sync On' : '🔄 Sync Off'}
                    </button>

                    {syncPreview && (
                        <div className="sync-settings">
                            <div className="sync-setting-item">
                                <label>Division:</label>
                                <select
                                    className="sync-select"
                                    value={quantizeDivision}
                                    onChange={e => setQuantizeDivision(e.target.value)}
                                >
                                    <option value="1Bar">1 Bar</option>
                                    <option value="1/2">1/2</option>
                                    <option value="1/4">1/4</option>
                                    <option value="1/8">1/8</option>
                                    <option value="1/16">1/16</option>
                                </select>
                            </div>
                            <div className="sync-setting-item">
                                <label>Strength:</label>
                                <select
                                    className="sync-select"
                                    value={quantizeStrength}
                                    onChange={e => setQuantizeStrength(parseFloat(e.target.value))}
                                >
                                    <option value={1.0}>100%</option>
                                    <option value={0.5}>50%</option>
                                    <option value={0.25}>25%</option>
                                    <option value={0.1}>10%</option>
                                </select>
                            </div>
                            <div className="sync-setting-item">
                                <label>Swing:</label>
                                <input
                                    type="range"
                                    min="0"
                                    max="1"
                                    step="0.01"
                                    value={swing}
                                    onChange={e => setSwing(parseFloat(e.target.value))}
                                    className="sync-slider"
                                />
                                <span className="sync-value">{Math.round(swing * 100)}%</span>
                            </div>
                        </div>
                    )}
                </div>
            </div>

            <div className="library-header">
                <div className="library-tabs">
                    <button className={tab === 'samples' ? 'active' : ''} onClick={() => setTab('samples')}>SAMPLES</button>
                    <button className={tab === 'plugins' ? 'active' : ''} onClick={() => setTab('plugins')}>PLUGINS</button>
                    <button className={tab === 'native' ? 'active' : ''} onClick={() => setTab('native')}>NATIVE</button>
                    <button className={tab === 'generators' ? 'active' : ''} onClick={() => setTab('generators')}>GENERATORS</button>
                </div>
                <div className="library-tools">
                    {tab === 'samples' && (
                        <>
                            <button className="btn-icon" onClick={handleImport} title="Import File">📄</button>
                            <button className="btn-icon" onClick={handleAddFolder} title="Add Folder">📁</button>
                        </>
                    )}
                    {tab === 'plugins' && (
                        <>
                            <button className="btn-icon" onClick={handleImportPlugin} title="Import Plugin">📂</button>
                            <button className="btn-icon" onClick={handleScanPlugins} title="Rescan Plugins">🔄</button>
                        </>
                    )}
                </div>
            </div>

            {tab === 'samples' ? (
                items.length > 0 ? (
                    <VirtualScroll
                        items={items}
                        itemHeight={90}
                        renderItem={(item, style) => (
                            <LibraryItem
                                key={item.id}
                                item={item}
                                type="clip"
                                onPreviewStart={handlePreviewStart}
                                onPreviewStop={handlePreviewStop}
                                style={style}
                            />
                        )}
                    />
                ) : <div className="library-items"><div className="empty-state">No samples found. Import or add directory.</div></div>
            ) : tab === 'generators' ? (
                <GeneratorsPanel />
            ) : (
                <PluginBrowser onLoadPlugin={handleLoadPlugin} />
            )}
        </div>
    );
};
