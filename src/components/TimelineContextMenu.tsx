import React, { useEffect, useRef } from 'react';
import './TimelineContextMenu.css';

interface TimelineContextMenuProps {
    x: number;
    y: number;
    onClose: () => void;
    onAction: (action: string, payload?: any) => void;
}

export const TimelineContextMenu: React.FC<TimelineContextMenuProps> = ({ x, y, onClose, onAction }) => {
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
            <div className="menu-header">Project Operations</div>

            <div className="menu-item" onClick={() => onAction('insert_track_audio')}>
                <span className="icon">🎤</span> Insert Audio Track
            </div>
            <div className="menu-item" onClick={() => onAction('insert_track_midi')}>
                <span className="icon">🎹</span> Insert MIDI Track
            </div>
            <div className="menu-item" onClick={() => onAction('insert_track_folder')}>
                <span className="icon">📁</span> Insert Folder Track
            </div>

            <div className="menu-separator" />

            <div className="menu-item has-submenu">
                <span className="icon">🔁</span> Auto-Loop Snapping
                <div className="submenu">
                    <div className="menu-header" style={{ fontSize: '9px', padding: '4px 8px' }}>Loop Lengths</div>
                    <div className="menu-item" onClick={() => onAction('auto_loop', { bars: 1 })}>1 Bar</div>
                    <div className="menu-item" onClick={() => onAction('auto_loop', { bars: 2 })}>2 Bars</div>
                    <div className="menu-item" onClick={() => onAction('auto_loop', { bars: 4 })}>4 Bars</div>
                    <div className="menu-item" onClick={() => onAction('auto_loop', { bars: 8 })}>8 Bars</div>
                    <div className="menu-separator" />
                    <div className="menu-item" onClick={() => onAction('auto_loop', { beatFraction: 0.25 })}>1/4 Beat</div>
                    <div className="menu-item" onClick={() => onAction('auto_loop', { beatFraction: 0.125 })}>1/8 Beat</div>
                    <div className="menu-item" onClick={() => onAction('auto_loop', { beatFraction: 0.0625 })}>1/16 Beat</div>
                    <div className="menu-item" onClick={() => onAction('auto_loop', { beatFraction: 0.0078125 })}>1/128 Beat</div>
                    <div className="menu-separator" />
                    <div className="menu-item" onClick={() => onAction('snap_loop_zero_crossing')}>
                        <span className="icon">🧲</span> Snap to Zero-Crossing
                    </div>
                </div>
            </div>

            <div className="menu-item" onClick={() => onAction('tempo_detective')}>
                <span className="icon">📐</span> Tempo Detective (Analyze Region)
            </div>

            <div className="menu-separator" />

            <div className="menu-item" onClick={() => onAction('insert_silence')}>
                <span className="icon">⏱️</span> Insert Silence at Playhead
            </div>
            <div className="menu-item" onClick={() => onAction('delete_time')}>
                <span className="icon">✂️</span> Delete Selected Time
            </div>
            <div className="menu-item" onClick={() => onAction('duplicate_time')}>
                <span className="icon">👯</span> Duplicate Selected Time
            </div>
            <div className="menu-item" onClick={() => onAction('paste_time')}>
                <span className="icon">📋</span> Paste Time
            </div>
            <div className="menu-item" onClick={() => onAction('set_time_signature')}>
                <span className="icon">⏱️</span> Set Time Signature
            </div>

            <div className="menu-separator" />

            <div className="menu-item" onClick={() => onAction('zoom_to_selection')}>
                <span className="icon">🔍</span> Zoom to Selection
            </div>
            <div className="menu-item" onClick={() => onAction('set_loop_range')}>
                <span className="icon">🔁</span> Set Loop to Selection
            </div>
        </div>
    );
};

