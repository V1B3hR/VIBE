import * as React from 'react';

interface TimelineToolbarProps {
    snap: number;
    setSnap: (snap: number) => void;
    automationMode: 'read' | 'draw' | 'erase';
    setAutomationMode: (mode: 'read' | 'draw' | 'erase') => void;
    handleSplit: () => void;
    handleUndo: () => void;
    handleRedo: () => void;
    setPixelsPerSample: React.Dispatch<React.SetStateAction<number>>;
    followPlayback: boolean;
    setFollowPlayback: (val: boolean) => void;
    setSelectedClips: (val: Set<string>) => void;
    swing: number;
    handleSetSwing: (val: number) => void;
}

export const TimelineToolbar: React.FC<TimelineToolbarProps> = ({
    snap,
    setSnap,
    automationMode,
    setAutomationMode,
    handleSplit,
    handleUndo,
    handleRedo,
    setPixelsPerSample,
    followPlayback,
    setFollowPlayback,
    setSelectedClips,
    swing,
    handleSetSwing
}) => {
    return (
        <div className="timeline-toolbar">
            <div className="toolbar-group">
                <span className="label">SNAP:</span>
                <select className="snap-select" value={snap} onChange={e => setSnap(parseInt(e.target.value))} data-testid="timeline-snap-select">
                    <option value={0}>Off</option>
                    <option value={1}>1/4</option>
                    <option value={2}>1/8</option>
                    <option value={4}>1/16</option>
                    <option value={8}>1/32</option>
                    <option value={16}>1/64</option>
                    <option value={32}>1/128</option>
                </select>
            </div>
            <div className="toolbar-group">
                <span className="label">TOOL:</span>
                <select className="snap-select" value={automationMode} onChange={e => setAutomationMode(e.target.value as any)} data-testid="timeline-tool-select">
                    <option value="read">Pointer/Read</option>
                    <option value="draw">Draw (Pencil)</option>
                    <option value="erase">Erase</option>
                </select>
            </div>
            <div className="toolbar-group">
                <span className="label">SWING:</span>
                <input
                    type="range"
                    className="swing-slider-global"
                    min="0"
                    max="1"
                    step="0.01"
                    value={swing}
                    onChange={e => handleSetSwing(parseFloat(e.target.value))}
                    title={`Swing: ${Math.round(swing * 100)}%`}
                />
                <span className="swing-value-global">{Math.round(swing * 100)}%</span>
            </div>
            <div className="toolbar-group">
                <button className="toolbar-btn" onClick={handleSplit} title="Split at Playhead (S)" data-testid="timeline-split-btn">✂️ Split</button>
                <button className="toolbar-btn" onClick={handleUndo} title="Undo (Ctrl+Z)" data-testid="timeline-undo-btn">↶ Undo</button>
                <button className="toolbar-btn" onClick={handleRedo} title="Redo (Ctrl+Shift+Z)" data-testid="timeline-redo-btn">↷ Redo</button>
                <button className="toolbar-btn" onClick={() => setPixelsPerSample(p => p * 1.5)} data-testid="timeline-zoom-in-btn">Zoom In (+)</button>
                <button className="toolbar-btn" onClick={() => setPixelsPerSample(p => p / 1.5)} data-testid="timeline-zoom-out-btn">Zoom Out (-)</button>
            </div>
            <div className="toolbar-spacer" />
            <div className="toolbar-group">
                <button className={`toolbar-btn ${followPlayback ? 'active' : ''}`} onClick={() => setFollowPlayback(!followPlayback)} title="Follow Playback (F)">🔊 Follow</button>
                <button className="toolbar-btn" onClick={() => setSelectedClips(new Set())} data-testid="timeline-deselect-all-btn">Deselect All</button>
            </div>
        </div>
    );
};
