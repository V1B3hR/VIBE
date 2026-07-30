import React, { useEffect, useRef } from 'react';
import './ClipContextMenu.css';

interface ClipContextMenuProps {
    x: number;
    y: number;
    clipId: string;
    trackIndex: number;
    isMidi: boolean;
    onClose: () => void;
    onAction: (action: string, payload?: any) => void;
}

export const ClipContextMenu: React.FC<ClipContextMenuProps> = ({ x, y, clipId, trackIndex, isMidi, onClose, onAction }) => {
    const menuRef = useRef<HTMLDivElement>(null);

    // Close on click outside
    useEffect(() => {
        const handleClickOutside = (e: MouseEvent) => {
            if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
                onClose();
            }
        };
        document.addEventListener('mousedown', handleClickOutside);
        return () => document.removeEventListener('mousedown', handleClickOutside);
    }, [onClose]);

    // Prevent going off-screen
    const style: React.CSSProperties = {
        top: y,
        left: x,
    };

    // Adjust if too close to bottom/right (Basic check)
    if (y > window.innerHeight - 300) style.top = y - 300;
    if (x > window.innerWidth - 200) style.left = x - 200;

    return (
        <div className="clip-context-menu glass" ref={menuRef} style={style}>
            <div className="menu-header">Clip Options</div>

            <div className="menu-item" onClick={() => onAction('rename')}>
                <span className="icon">✏️</span> Rename
            </div>

            <div className="menu-separator" />

            <div className="menu-item" onClick={() => onAction('cut')}>
                <span className="icon">✂️</span> Cut
            </div>
            <div className="menu-item" onClick={() => onAction('copy')}>
                <span className="icon">📋</span> Copy
            </div>
            <div className="menu-item" onClick={() => onAction('duplicate')}>
                <span className="icon">👯</span> Duplicate
            </div>

            <div className="menu-separator" />

            {isMidi && (
                <>
                    <div className="menu-item" onClick={() => onAction('transpose')}>
                        <span className="icon">🎵</span> Transpose...
                    </div>

                    <div className="menu-item has-submenu">
                        <span className="icon">󰒟</span> Quantize
                        <div className="submenu">
                            <div className="menu-item" onClick={() => onAction('quantize', { division: 'Quarter' })}>1/4 Note</div>
                            <div className="menu-item" onClick={() => onAction('quantize', { division: 'Eighth' })}>1/8 Note</div>
                            <div className="menu-item" onClick={() => onAction('quantize', { division: 'Sixteenth' })}>1/16 Note</div>
                            <div className="menu-item" onClick={() => onAction('quantize', { division: 'ThirtySecond' })}>1/32 Note</div>
                            <div className="menu-separator" />
                            <div className="menu-item" onClick={() => onAction('quantize', { division: 'Triplet' })}>Triplet (1/8T)</div>
                        </div>
                    </div>

                    <div className="menu-item has-submenu">
                        <span className="icon">💃</span> Groove
                        <div className="submenu">
                            <div className="menu-item" onClick={() => onAction('apply_groove', { template: 'MPC 60' })}>MPC 60 Swing</div>
                            <div className="menu-item" onClick={() => onAction('apply_groove', { template: 'SP 1200' })}>SP 1200 Grit</div>
                            <div className="menu-item" onClick={() => onAction('apply_groove', { template: 'Logic Tight' })}>Logic Tight</div>
                        </div>
                    </div>

                    <div className="menu-item has-submenu">
                        <span className="icon">🎲</span> Humanize
                        <div className="submenu">
                            <div className="menu-item" onClick={() => onAction('humanize', { amount: 5, velocity: 10 })}>Subtle (5ms)</div>
                            <div className="menu-item" onClick={() => onAction('humanize', { amount: 15, velocity: 20 })}>Moderate (15ms)</div>
                            <div className="menu-item" onClick={() => onAction('humanize', { amount: 40, velocity: 40 })}>Drunk (40ms)</div>
                        </div>
                    </div>

                    <div className="menu-item" onClick={() => onAction('legato')}>
                        <span className="icon">↔️</span> Legato
                    </div>
                    <div className="menu-item" onClick={() => onAction('duplicate_notes')}>
                        <span className="icon">📄</span> Duplicate Selected Notes
                    </div>
                    <div className="menu-item" onClick={() => onAction('convert_to_audio')}>
                        <span className="icon">🎙️</span> Convert to Audio (Bounce)
                    </div>
                    <div className="menu-separator" />
                </>
            )}

            <div className="menu-item" onClick={() => onAction('consolidate')}>
                <span className="icon">📦</span> Consolidate
            </div>

            <div className="menu-item" onClick={() => onAction('snap_to_grid')}>
                <span className="icon">🔗</span> Snap Clip Length to Grid
            </div>

            <div className="menu-item" onClick={() => onAction('statistics')}>
                <span className="icon">📊</span> Clip Statistics...
            </div>

            {!isMidi && (
                <>
                    <div className="menu-item" onClick={() => onAction('auto_crossfade')}>
                        <span className="icon">🪄</span> Auto-Crossfade at Boundaries
                    </div>

                    <div className="menu-item has-submenu">
                        <span className="icon">🎚️</span> Quick Gain
                        <div className="submenu">
                            <div className="menu-item" onClick={() => onAction('quick_gain', { gain: 6.0 })}>+6.0 dB</div>
                            <div className="menu-item" onClick={() => onAction('quick_gain', { gain: 3.0 })}>+3.0 dB</div>
                            <div className="menu-item" onClick={() => onAction('quick_gain', { gain: -3.0 })}>-3.0 dB</div>
                            <div className="menu-item" onClick={() => onAction('quick_gain', { gain: -6.0 })}>-6.0 dB</div>
                            <div className="menu-separator" />
                            <div className="menu-item" onClick={() => onAction('quick_gain', { gain: 0.0 })}>Reset (0 dB)</div>
                        </div>
                    </div>

                    <div className="menu-item has-submenu">
                        <span className="icon">🌊</span> Waveform Display Mode
                        <div className="submenu">
                            <div className="menu-item" onClick={() => onAction('set_display_mode', { mode: 'Bars' })}>Classic Bars</div>
                            <div className="menu-item" onClick={() => onAction('set_display_mode', { mode: 'Oscilloscope' })}>Analog Oscilloscope</div>
                            <div className="menu-item" onClick={() => onAction('set_display_mode', { mode: 'Rectified' })}>Rectified (Positive Peaks)</div>
                            <div className="menu-item" onClick={() => onAction('set_display_mode', { mode: 'Spectrum' })}>Real-time Spectrum</div>
                        </div>
                    </div>

                    <div className="menu-item" onClick={() => onAction('normalize')}>
                        <span className="icon">📈</span> Normalize
                    </div>

                    <div className="menu-item" onClick={() => onAction('reverse')}>
                        <span className="icon">◀️</span> Reverse Audio
                    </div>
                </>
            )}

            {!isMidi && (
                <>
                    <div className="menu-header" style={{ fontSize: '10px', opacity: 0.7, padding: '4px 8px' }}>Warp Mode</div>
                    <div className="menu-item" onClick={() => onAction('set_warp_mode', { mode: 'Beats' })}>
                        <span className="icon">🥁</span> Beats
                    </div>
                    <div className="menu-item" onClick={() => onAction('set_warp_mode', { mode: 'Tones' })}>
                        <span className="icon">🎹</span> Tones
                    </div>
                    <div className="menu-item" onClick={() => onAction('set_warp_mode', { mode: 'Texture' })}>
                        <span className="icon">☁️</span> Texture
                    </div>
                    <div className="menu-item" onClick={() => onAction('set_warp_mode', { mode: 'Repitch' })}>
                        <span className="icon">🔁</span> Repitch
                    </div>
                    <div className="menu-item" onClick={() => onAction('set_warp_mode', { mode: 'Complex' })}>
                        <span className="icon">🌌</span> Complex
                    </div>

                    <div className="menu-separator" />
                    <div className="menu-header" style={{ fontSize: '10px', opacity: 0.7, padding: '4px 8px' }}>Audio Alchemy</div>
                    <div className="menu-item" onClick={() => onAction('convert_audio_to_midi')}>
                        <span className="icon">✨</span> Convert to MIDI
                    </div>
                    <div className="menu-item" onClick={() => onAction('extract_groove')}>
                        <span className="icon">💃</span> Extract Groove
                    </div>
                    <div className="menu-separator" />
                </>
            )}

            <div className="menu-separator" />

            {!isMidi && (
                <div className="menu-item" onClick={() => onAction('show_in_explorer')}>
                    <span className="icon">📂</span> Show in Explorer
                </div>
            )}

            <div className="menu-item" onClick={() => onAction('export_audio')}>
                <span className="icon">🎵</span> Export Audio...
            </div>

            <div className="menu-item" onClick={() => onAction('export_midi')}>
                <span className="icon">🎹</span> {isMidi ? "Export MIDI..." : "Convert to MIDI..."}
            </div>

            <div className="menu-separator" />

            <div className="menu-section-label">Clip Color</div>
            <div className="clip-color-palette">
                {[
                    '#ff4d4d', '#ff8c42', '#ffd700', '#a8e063',
                    '#4dd0e1', '#4a9eff', '#9b59b6', '#e91e8c',
                    '#ff6b9d', '#00bcd4', '#69f0ae', '#fff176',
                    '#c8a96e', '#b0bec5', '#ff7043', '#ce93d8',
                ].map(c => (
                    <div
                        key={c}
                        className="clip-color-swatch"
                        style={{ background: c }}
                        title={c}
                        onClick={() => { onAction('set_color', { color: c }); onClose(); }}
                    />
                ))}
                <label className="clip-color-custom" title="Custom color">
                    🎨
                    <input
                        type="color"
                        style={{ position: 'absolute', opacity: 0, width: 0, height: 0 }}
                        onChange={(e) => { onAction('set_color', { color: e.target.value }); onClose(); }}
                    />
                </label>
            </div>

            <div className="menu-separator" />

            <div className="menu-item delete" onClick={() => onAction('delete')}>
                <span className="icon">🗑️</span> Delete
            </div>
        </div>
    );
};
