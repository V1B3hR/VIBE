import React, { useEffect, useRef } from 'react';
import './TimelineContextMenu.css'; // Re-use the same styles

interface TrackContextMenuProps {
    x: number;
    y: number;
    trackIndex: number;
    trackId: string;
    onClose: () => void;
    onAction: (action: string, payload?: any) => void;
}

export const TrackContextMenu: React.FC<TrackContextMenuProps> = ({ x, y, trackIndex, trackId, onClose, onAction }) => {
    const menuRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        const handleClickOutside = (e: MouseEvent) => {
            if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
                onClose();
            }
        };
        document.addEventListener('mousedown', handleClickOutside);
        return () => document.removeEventListener('mousedown', handleClickOutside);
    }, [onClose]);

    const style: React.CSSProperties = {
        top: y,
        left: x,
        position: 'fixed',
        zIndex: 1000
    };

    return (
        <div className="timeline-context-menu glyph-menu glass" ref={menuRef} style={style}>
            <div className="menu-header">Track Options</div>

            <div className="menu-item" onClick={() => onAction('rename_track', { trackIndex })}>
                <span className="icon">✏️</span> Rename
            </div>
            <div className="menu-item" onClick={() => onAction('duplicate_track', { trackIndex })}>
                <span className="icon">👯</span> Duplicate
            </div>
            <div className="menu-item" onClick={() => onAction('change_track_color', { trackIndex })}>
                <span className="icon">🎨</span> Change Color
            </div>

            <div className="menu-separator" />

            <div className="menu-item" onClick={() => onAction('add_child_track', { trackIndex })}>
                <span className="icon">➕</span> Add Child Track
            </div>

            <div className="menu-separator" />

            <div className="menu-item" onClick={() => onAction('toggle_disable', { trackIndex })}>
                <span className="icon">🚫</span> Disable Track
            </div>
            <div className="menu-item" onClick={() => onAction('toggle_freeze', { trackIndex })}>
                <span className="icon">❄️</span> Toggle Freeze
            </div>
            <div className="menu-item" onClick={() => onAction('toggle_automation', { trackIndex })}>
                <span className="icon">📉</span> Toggle Automation Lane
            </div>

            <div className="menu-separator" />
            <div className="menu-header" style={{ fontSize: '10px', opacity: 0.7, padding: '4px 8px' }}>Automation Mode</div>
            <div className="menu-item" onClick={() => onAction('set_automation_mode', { trackIndex, mode: 'Read' })}>
                <span className="mode-dot" style={{ color: '#4caf50' }}>●</span> Read
            </div>
            <div className="menu-item" onClick={() => onAction('set_automation_mode', { trackIndex, mode: 'Write' })}>
                <span className="mode-dot" style={{ color: '#f44336' }}>●</span> Write
            </div>
            <div className="menu-item" onClick={() => onAction('set_automation_mode', { trackIndex, mode: 'Touch' })}>
                <span className="mode-dot" style={{ color: '#ff9800' }}>●</span> Touch
            </div>
            <div className="menu-item" onClick={() => onAction('set_automation_mode', { trackIndex, mode: 'Latch' })}>
                <span className="mode-dot" style={{ color: '#2196f3' }}>●</span> Latch
            </div>
            <div className="menu-item" onClick={() => onAction('set_automation_mode', { trackIndex, mode: 'Off' })}>
                <span className="mode-dot" style={{ color: '#bbb' }}>●</span> Off
            </div>
            <div className="menu-separator" />
            <div className="menu-item" onClick={() => onAction('arm_track', { trackIndex })}>
                <span className="icon">🔴</span> Arm for Recording
            </div>


            <div className="menu-item delete" onClick={() => onAction('delete_track', { trackIndex })}>
                <span className="icon">🗑️</span> Delete Track
            </div>
        </div>
    );
};
