import React from 'react';
import { PluginInfo } from '../types/plugin';
import { formatRelativeTimeFromSeconds } from '../utils/timeFormatting';
import './PluginCard.css';

interface PluginCardProps {
    plugin: PluginInfo;
    isActive?: boolean;
    onSelect?: (plugin: PluginInfo) => void;
    onToggleFavorite?: (pluginId: string) => void;
    onDragStart?: (e: React.DragEvent, plugin: PluginInfo) => void;
    onContextMenu?: (e: React.MouseEvent, plugin: PluginInfo) => void;
}

export const PluginCard: React.FC<PluginCardProps> = ({
    plugin,
    isActive,
    onSelect,
    onToggleFavorite,
    onDragStart,
    onContextMenu
}) => {
    const handleDragStart = (e: React.DragEvent) => {
        if (onDragStart) {
            onDragStart(e, plugin);
        }
        // Set drag ghost image or data
        e.dataTransfer.setData('vibe/plugin', JSON.stringify(plugin));
        e.dataTransfer.effectAllowed = 'copy';
    };

    const handleClick = () => {
        if (onSelect) {
            onSelect(plugin);
        }
    };

    const handleFavoriteClick = (e: React.MouseEvent) => {
        e.stopPropagation();
        if (onToggleFavorite) {
            onToggleFavorite(plugin.id);
        }
    };

    const handleRightClick = (e: React.MouseEvent) => {
        if (onContextMenu) {
            e.preventDefault();
            onContextMenu(e, plugin);
        }
    };

    return (
        <div
            className={`plugin-card ${isActive ? 'active' : ''} ${plugin.is_blacklisted ? 'blacklisted' : ''}`}
            onClick={handleClick}
            onContextMenu={handleRightClick}
            draggable={!plugin.is_blacklisted}
            onDragStart={handleDragStart}
        >
            <div className="plugin-card-main">
                <div className="plugin-icon">
                    {plugin.category === 'Instrument' ? '🎹' : '🎚️'}
                </div>
                <div className="plugin-info-cols">
                    <div className="plugin-name-row">
                        <span className="plugin-name" title={plugin.name}>{plugin.name}</span>
                        {plugin.is_favorite && <span className="favorite-star" onClick={handleFavoriteClick}>⭐</span>}
                        {!plugin.is_favorite && <span className="favorite-star empty" onClick={handleFavoriteClick}>☆</span>}
                    </div>
                    <div className="plugin-meta-row">
                        <span className="plugin-vendor">{plugin.vendor}</span>
                        <span className={`plugin-type-badge ${plugin.plugin_type.toLowerCase()}`}>
                            {plugin.plugin_type}
                        </span>
                    </div>
                </div>
            </div>

            <div className="plugin-card-footer">
                <div className="plugin-tags-row">
                    <span className="plugin-category-tag">{plugin.category}</span>
                    {plugin.tags.slice(0, 2).map(tag => (
                        <span key={tag} className="plugin-user-tag">{tag}</span>
                    ))}
                    {plugin.tags.length > 2 && <span className="plugin-user-tag more">+{plugin.tags.length - 2}</span>}
                </div>
                {plugin.last_used && (
                    <span className="last-used-time" title={`Last used: ${new Date(plugin.last_used * 1000).toLocaleString()}`}>
                        🕒 {formatRelativeTimeFromSeconds(plugin.last_used)}
                    </span>
                )}
                <div className="performance-stats">
                    {plugin.cpu_usage_avg !== undefined && (
                        <span className="perf-chip cpu" title="Average CPU Usage">
                            {(plugin.cpu_usage_avg ?? 0).toFixed(1)}%
                        </span>
                    )}
                    {plugin.latency_samples !== undefined && plugin.latency_samples > 0 && (
                        <span className="perf-chip latency" title="Latence (samples)">
                            {plugin.latency_samples} spls
                        </span>
                    )}
                </div>
                {plugin.is_blacklisted && <span className="blacklist-badge">CRASHED</span>}
                {plugin.hidden && <span className="status-badge hidden">HIDDEN</span>}
                {plugin.deprecated && <span className="status-badge deprecated">LEGACY</span>}
            </div>
        </div>
    );
};
