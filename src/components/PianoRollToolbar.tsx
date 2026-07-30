import React from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
    MidiClip,
    TrackInfo,
    ColorMode,
    Tool,
    GROOVE_TEMPLATES,
    NOTE_NAMES,
    SCALE_INTERVALS,
    ScaleType
} from './PianoRollTypes';

interface PianoRollToolbarProps {
    trackIdx: number;
    currentClipId: string;
    setCurrentClipId: (id: string) => void;
    clip: MidiClip | null;
    ghostClips: MidiClip[];
    showGhostNotes: boolean;
    setShowGhostNotes: (val: boolean) => void;
    overlayTrackId: string;
    setOverlayTrackId: (id: string) => void;
    tracks: TrackInfo[];
    colorMode: ColorMode;
    setColorMode: (val: ColorMode) => void;
    groove: string;
    applyGroove: (val: string) => Promise<void>;
    isRecordingMacro: boolean;
    setIsRecordingMacro: (val: boolean) => void;
    macroBuffer: { command: string; args: any }[];
    setMacroBuffer: React.Dispatch<React.SetStateAction<{ command: string; args: any }[]>>;
    savedMacros: { name: string; steps: { command: string; args: any }[] }[];
    setSavedMacros: React.Dispatch<React.SetStateAction<{ name: string; steps: { command: string; args: any }[] }[]>>;
    tool: Tool;
    setTool: (val: Tool) => void;
    snap: number;
    setSnap: (val: number) => void;
    runAction: (command: string, args: any) => Promise<any>;
    performCommand: (action: string, payload?: any) => Promise<void>;
    loadData: () => void;
    followPlayhead: boolean;
    setFollowPlayhead: React.Dispatch<React.SetStateAction<boolean>>;
    isFolded: boolean;
    setIsFolded: React.Dispatch<React.SetStateAction<boolean>>;
    setZoom: React.Dispatch<React.SetStateAction<{ x: number; y: number }>>;
    setShowArpeggiator: (val: boolean) => void;
    startMidiLearn: (paramId: string) => void;
    onClose: () => void;
}

export function PianoRollToolbar({
    trackIdx,
    currentClipId,
    setCurrentClipId,
    clip,
    ghostClips,
    showGhostNotes,
    setShowGhostNotes,
    overlayTrackId,
    setOverlayTrackId,
    tracks,
    colorMode,
    setColorMode,
    groove,
    applyGroove,
    isRecordingMacro,
    setIsRecordingMacro,
    macroBuffer,
    setMacroBuffer,
    savedMacros,
    setSavedMacros,
    tool,
    setTool,
    snap,
    setSnap,
    runAction,
    performCommand,
    loadData,
    followPlayhead,
    setFollowPlayhead,
    isFolded,
    setIsFolded,
    setZoom,
    setShowArpeggiator,
    startMidiLearn,
    onClose
}: PianoRollToolbarProps) {
    return (
        <div className="piano-roll-toolbar">
            <div className="toolbar-group">
                <span className="toolbar-label">CLIP:</span>
                <select
                    className="toolbar-select"
                    style={{ width: '150px', fontWeight: 'bold', color: '#fff', background: '#222' }}
                    value={currentClipId}
                    onChange={(e) => {
                        setCurrentClipId(e.target.value);
                    }}
                >
                    <option value={currentClipId}>{clip?.name || 'Current Clip'}</option>
                    {ghostClips.map(c => (
                        <option key={c.id} value={c.id}>{c.name}</option>
                    ))}
                </select>
                <button
                    className={`toolbar-btn ${showGhostNotes ? 'active' : ''}`}
                    onClick={() => setShowGhostNotes(!showGhostNotes)}
                    title="Show Ghost Notes (Other clips on track)"
                >
                    👻
                </button>
            </div>

            <div className="toolbar-group">
                <span className="toolbar-label">WAVE:</span>
                <select
                    className="toolbar-select"
                    style={{ width: '100px', background: overlayTrackId ? '#345' : '#222' }}
                    value={overlayTrackId || ""}
                    onChange={(e) => setOverlayTrackId(e.target.value)}
                >
                    <option value="">None</option>
                    {tracks.map(t => (
                        <option key={t.id} value={t.id}>{t.name}</option>
                    ))}
                </select>
            </div>

            <div className="toolbar-group">
                <span className="toolbar-label">COLOR:</span>
                <select
                    className="toolbar-select"
                    style={{ width: '80px', background: '#222' }}
                    value={colorMode}
                    onChange={(e) => setColorMode(e.target.value as ColorMode)}
                >
                    <option value="clip">Clip</option>
                    <option value="channel">Channel</option>
                    <option value="velocity">Velocity</option>
                    <option value="pitch">Pitch</option>
                </select>
            </div>

            <div className="toolbar-group">
                <span className="toolbar-label">GROOVE:</span>
                <select
                    className="toolbar-select"
                    style={{ width: '100px', background: '#222' }}
                    value={groove}
                    onChange={(e) => applyGroove(e.target.value)}
                >
                    {Object.keys(GROOVE_TEMPLATES).map(k => (
                        <option key={k} value={k}>{k}</option>
                    ))}
                </select>
            </div>

            <div className="toolbar-group">
                <span className="toolbar-label">PATTERN:</span>
                <input
                    key={clip?.id}
                    type="text"
                    defaultValue={clip?.pattern_id || ""}
                    style={{ width: '60px', background: '#222', color: '#fff', border: '1px solid #444', fontSize: '12px', padding: '2px' }}
                    onBlur={(e) => {
                        if (!clip) return;
                        const val = e.target.value.trim();
                        const newId = val.length > 0 ? val : null;
                        const updatedClip = { ...clip, pattern_id: newId };
                        invoke('update_midi_clip', {
                            trackIdx,
                            clipId: currentClipId,
                            clip: updatedClip
                        }).then(loadData);
                    }}
                    onKeyDown={(e) => {
                        if (e.key === 'Enter') e.currentTarget.blur();
                    }}
                    title="Linked Pattern ID. Clips with same ID share edits."
                />
            </div>

            <div className="toolbar-group">
                <span className="toolbar-label">TUNING:</span>
                <select
                    className="toolbar-select"
                    style={{ width: '60px', background: '#222' }}
                    value={clip?.tuning_steps || 12}
                    onChange={(e) => {
                        if (!clip) return;
                        const steps = parseInt(e.target.value);
                        const updatedClip = { ...clip, tuning_steps: steps };
                        invoke('update_midi_clip', {
                            trackIdx,
                            clipId: currentClipId,
                            clip: updatedClip
                        }).then(loadData);
                    }}
                >
                    <option value="12">12-TET</option>
                    <option value="19">19-TET</option>
                    <option value="24">24-TET</option>
                    <option value="31">31-TET</option>
                    <option value="48">48-TET</option>
                </select>
            </div>

            <div className="toolbar-group">
                <button
                    className="toolbar-btn"
                    style={{ color: '#ff6644', fontWeight: 'bold' }}
                    onClick={async () => {
                        if (!clip) return;
                        try {
                            await invoke('generate_stress_notes', {
                                trackIdx,
                                clipId: currentClipId,
                                count: 5000
                            });
                            loadData();
                        } catch (e) {
                            console.error("Stress Test Failed:", e);
                        }
                    }}
                >
                    🚀 STRESS TEST (5000)
                </button>
            </div>

            <div className="toolbar-group">
                <span className="toolbar-label">TIME SIG:</span>
                <input
                    type="number"
                    min="1"
                    max="32"
                    className="toolbar-input"
                    style={{ width: '40px', background: '#222', color: '#fff', border: '1px solid #444', textAlign: 'center' }}
                    value={clip?.time_signature_num || 4}
                    onChange={(e) => {
                        if (!clip) return;
                        const num = parseInt(e.target.value);
                        const updatedClip = { ...clip, time_signature_num: num };
                        invoke('update_midi_clip', {
                            trackIdx, clipId: currentClipId, clip: updatedClip
                        }).then(loadData);
                    }}
                />
                <span style={{ color: '#888' }}>/</span>
                <select
                    className="toolbar-select"
                    style={{ width: '40px', background: '#222' }}
                    value={clip?.time_signature_den || 4}
                    onChange={(e) => {
                        if (!clip) return;
                        const den = parseInt(e.target.value);
                        const updatedClip = { ...clip, time_signature_den: den };
                        invoke('update_midi_clip', {
                            trackIdx, clipId: currentClipId, clip: updatedClip
                        }).then(loadData);
                    }}
                >
                    <option value="2">2</option>
                    <option value="4">4</option>
                    <option value="8">8</option>
                    <option value="16">16</option>
                </select>
            </div>

            <div className="toolbar-group">
                <span className="toolbar-label">TRACK:</span>
                <button
                    className="toolbar-btn"
                    style={{ fontSize: '10px' }}
                    onClick={() => {
                        const t = tracks[trackIdx];
                        if (t && t.volume) startMidiLearn(t.volume.id);
                    }}
                    title="Learn MIDI mapping for Track Volume"
                >
                    Learn Vol
                </button>
            </div>

            <div className="toolbar-group">
                <button
                    className={`toolbar-btn ${isRecordingMacro ? 'recording-btn' : ''}`}
                    style={isRecordingMacro ? { color: 'red', borderColor: 'red' } : {}}
                    onClick={() => {
                        if (isRecordingMacro) {
                            setIsRecordingMacro(false);
                            if (macroBuffer.length > 0) {
                                const name = prompt("Save Macro As:", `Macro ${savedMacros.length + 1}`);
                                if (name) {
                                    setSavedMacros(prev => [...prev, { name, steps: macroBuffer }]);
                                }
                            }
                            setMacroBuffer([]);
                        } else {
                            setIsRecordingMacro(true);
                            setMacroBuffer([]);
                        }
                    }}
                    title={isRecordingMacro ? "Stop Recording" : "Record Macro"}
                    data-testid="piano-roll-macro-record-btn"
                >
                    {isRecordingMacro ? '⏹' : '⏺'}
                </button>

                {savedMacros.length > 0 && (
                    <select
                        className="toolbar-select"
                        style={{ width: '100px', color: '#88ff88' }}
                        onChange={(e) => {
                            if (e.target.value === "") return;
                            const macro = savedMacros.find(m => m.name === e.target.value);
                            if (macro) {
                                (async () => {
                                    for (const step of macro.steps) {
                                        if (step.command.startsWith('__META__:')) {
                                            await performCommand(step.command.replace('__META__:', ''), step.args);
                                        } else {
                                            await invoke(step.command, step.args);
                                        }
                                    }
                                    loadData();
                                })();
                            }
                            e.target.value = "";
                        }}
                    >
                        <option value="">▶ Play Macro...</option>
                        {savedMacros.map(m => <option key={m.name} value={m.name}>{m.name}</option>)}
                    </select>
                )}
            </div>

            <div className="toolbar-group">
                <button className={`toolbar-btn ${tool === 'select' ? 'active' : ''}`} onClick={() => setTool('select')} data-testid="tool-select-btn">↖ Select</button>
                <button className={`toolbar-btn ${tool === 'pencil' ? 'active' : ''}`} onClick={() => setTool('pencil')} data-testid="tool-pencil-btn">✎ Pencil</button>
                <button className={`toolbar-btn ${tool === 'brush' ? 'active' : ''}`} onClick={() => setTool('brush')} data-testid="tool-brush-btn">🖌 Brush</button>
                <button className={`toolbar-btn ${tool === 'eraser' ? 'active' : ''}`} onClick={() => setTool('eraser')} data-testid="tool-eraser-btn">⌫ Eraser</button>
            </div>

            <div className="toolbar-group">
                <select className="toolbar-select" value={snap} onChange={e => setSnap(parseInt(e.target.value))} data-testid="piano-roll-snap-select">
                    <option value={4}>1/4 note</option>
                    <option value={8}>1/8 note</option>
                    <option value={16}>1/16 note</option>
                    <option value={32}>1/32 note</option>
                </select>
                <button className="toolbar-btn" onClick={() => {
                    const division =
                        snap === 4 ? 'Quarter' :
                            snap === 8 ? 'Eighth' :
                                snap === 16 ? 'Sixteenth' :
                                    'ThirtySecond';
                    runAction('quantize_notes', { trackIdx, clipId: currentClipId, division }).then(loadData);
                }} data-testid="piano-roll-quantize-btn">Q</button>
            </div>

            <div className="toolbar-group">
                <select className="toolbar-select" style={{ width: '100px' }} value={clip?.groove_template || ''} onChange={(e) => {
                    runAction('apply_groove_template', { trackIdx, clipId: currentClipId, templateName: e.target.value }).then(loadData);
                }}>
                    <option value="">No Groove</option>
                    <option value="Swing 50%">Swing 50%</option>
                    <option value="Swing 58%">Swing 58%</option>
                </select>
            </div>

            <button className={`toolbar-btn ${followPlayhead ? 'active' : ''}`} onClick={() => setFollowPlayhead(f => !f)} data-testid="piano-roll-follow-btn">➡ Follow</button>
            <button className={`toolbar-btn ${isFolded ? 'active' : ''}`} onClick={() => setIsFolded(f => !f)} title="Fold: Only show keys with notes">🎹 Fold</button>
            <button className="toolbar-btn" onClick={() => setZoom(z => ({ ...z, x: z.x * 1.2 }))} data-testid="piano-roll-zoom-in-btn">🔍+</button>
            <button className="toolbar-btn" onClick={() => setZoom(z => ({ ...z, x: z.x / 1.2 }))} data-testid="piano-roll-zoom-out-btn">🔍-</button>

            <div className="toolbar-group">
                <select
                    className="toolbar-select"
                    style={{ width: '60px' }}
                    value={clip?.scale?.root || 0}
                    onChange={(e) => {
                        if (clip) {
                            runAction('set_clip_scale', {
                                trackIdx,
                                clipId: currentClipId,
                                scale: { root: parseInt(e.target.value), type: clip.scale?.type || 'Minor' }
                            }).then(loadData);
                        }
                    }}
                >
                    {NOTE_NAMES.map((note, i) => (
                        <option key={note} value={i}>{note}</option>
                    ))}
                </select>
                <select
                    className="toolbar-select"
                    style={{ width: '120px' }}
                    value={clip?.scale?.type || 'Minor'}
                    onChange={(e) => {
                        if (clip) {
                            runAction('set_clip_scale', {
                                trackIdx,
                                clipId: currentClipId,
                                scale: { root: clip.scale?.root || 0, type: e.target.value }
                            }).then(loadData);
                        }
                    }}
                >
                    {Object.keys(SCALE_INTERVALS).map(s => (
                        <option key={s} value={s}>{s}</option>
                    ))}
                </select>

                <button className="toolbar-btn" onClick={() => {
                    runAction('detect_chords', { trackIdx, clipId: currentClipId });
                    setTimeout(loadData, 100);
                }}>🪄 Detect Chords</button>

                <button className="toolbar-btn" onClick={() => setShowArpeggiator(true)} data-testid="piano-roll-arpeggiator-btn">
                    🎹 Arpeggiator
                </button>
            </div>

            <button className="close-btn" onClick={onClose} data-testid="piano-roll-close-btn">Finish Editing</button>
        </div>
    );
}
