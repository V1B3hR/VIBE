import React, { useEffect, useRef } from 'react';
import { PluginInfo } from '../types/plugin';
import './PluginContextMenu.css';

interface PluginContextMenuProps {
    x: number;
    y: number;
    plugin: PluginInfo;
    onClose: () => void;
    onAction: (action: string, plugin: PluginInfo, payload?: any) => void;
}

export const PluginContextMenu: React.FC<PluginContextMenuProps> = ({
    x, y, plugin, onClose, onAction
}) => {
    const menuRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        const handleClickOutside = (e: MouseEvent) => {
            if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
                onClose();
            }
        };
        const handleScroll = () => onClose();

        document.addEventListener('mousedown', handleClickOutside);
        window.addEventListener('scroll', handleScroll, true);

        return () => {
            document.removeEventListener('mousedown', handleClickOutside);
            window.removeEventListener('scroll', handleScroll, true);
        };
    }, [onClose]);

    const style: React.CSSProperties = {
        top: y,
        left: x,
    };

    // Screen boundary check
    const menuWidth = 200;
    const menuHeight = 350; // Max expected
    if (x + menuWidth > window.innerWidth) style.left = x - menuWidth;
    if (y + menuHeight > window.innerHeight) style.top = y - menuHeight;

    return (
        <div className="plugin-context-menu glass" ref={menuRef} style={style} onContextMenu={(e) => e.preventDefault()}>
            <div className="menu-header">
                <span className="plugin-name-hint">{plugin.name}</span>
            </div>

            <div className="menu-item" onClick={() => onAction('load', plugin)}>
                <span className="icon">🚀</span> Load Plugin
            </div>

            <div className="menu-item" onClick={() => onAction('toggle_favorite', plugin)}>
                <span className="icon">{plugin.is_favorite ? '☆' : '⭐'}</span>
                {plugin.is_favorite ? 'Remove from Favorites' : 'Add to Favorites'}
            </div>

            <div className="menu-separator" />

            <div className="menu-item" onClick={() => onAction('show_in_folder', plugin)}>
                <span className="icon">📂</span> Show in Explorer
            </div>

            <div className="menu-item" onClick={() => onAction('set_folder', plugin)}>
                <span className="icon">📁</span> {plugin.custom_folder ? 'Change Folder' : 'Assign to Folder'}
            </div>

            <div className="menu-item" onClick={() => onAction('show_presets', plugin)}>
                <span className="icon">📋</span> List Presets (Future)
            </div>

            <div className="menu-separator" />

            {/* Tag Management Submenu Hint */}
            <div className="menu-item disabled">
                <span className="icon">🏷️</span> Manage Tags...
            </div>

            <div className="menu-item" onClick={() => onAction('rename', plugin)}>
                <span className="icon">✏️</span> Rename (Local)
            </div>

            <div className="menu-separator" />

            <div className="menu-item delete" onClick={() => onAction('blacklist', plugin)}>
                <span className="icon">🚫</span> Blacklist Plugin
            </div>

            <div className="menu-item" onClick={() => onAction('rescan_single', plugin)}>
                <span className="icon">🔄</span> Rescan This Plugin
            </div>

            <div className="menu-separator" />

            <div className="menu-item" onClick={() => onAction('set_hidden', plugin, !plugin.hidden)}>
                <span className="icon">{plugin.hidden ? '👁️' : '👁️‍🗨️'}</span>
                {plugin.hidden ? 'Unhide Plugin' : 'Hide Plugin'}
            </div>

            <div className="menu-item" onClick={() => onAction('set_deprecated', plugin, !plugin.deprecated)}>
                <span className="icon">⚠️</span>
                {plugin.deprecated ? 'Remove Legacy Mark' : 'Mark as Legacy'}
            </div>

            <div className="menu-item" onClick={() => onAction('merge_duplicates', plugin)}>
                <span className="icon">🧬</span> Merge Duplicates
            </div>

            <div className="menu-item" onClick={() => onAction('show_diagnostics', plugin)}>
                <span className="icon">🩺</span> Show Diagnostics
            </div>
        </div>
    );
};
