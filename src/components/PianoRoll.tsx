import { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './PianoRoll.css';
import { ArpeggiatorModal } from './ArpeggiatorModal';
import { PianoRollToolbar } from './PianoRollToolbar';
import { PianoRollGrid } from './PianoRollGrid';
import {
    MidiNote,
    MidiClip,
    TrackInfo,
    Tool,
    ColorMode,
    CHANNEL_COLORS,
    getNoteName
} from './PianoRollTypes';

interface PianoRollProps {
    trackIdx: number;
    clipId: string;
    onClose: () => void;
}

const SAMPLE_RATE = 44100;
const BPM = 120;
const SAMPLES_PER_BEAT = (SAMPLE_RATE * 60) / BPM;

export function PianoRoll({ trackIdx, clipId, onClose }: PianoRollProps) {
    // ------------------------------------------------------------------------
    // STATE
    // ------------------------------------------------------------------------
    const [currentClipId, setCurrentClipId] = useState(clipId);
    const [clip, setClip] = useState<MidiClip | null>(null);
    const [selection, setSelection] = useState<Set<number>>(new Set());
    const [tool, setTool] = useState<Tool>('select');
    const [snap, setSnap] = useState(16);
    const [zoom, setZoom] = useState({ x: 1.0, y: 1.0 });
    const [scroll, setScroll] = useState({ x: 0, y: 0 });
    const [hoverInfo, setHoverInfo] = useState<{ note: number; time: number } | null>(null);
    const [playhead, setPlayhead] = useState(0);
    const [clipboard, setClipboard] = useState<MidiNote[]>([]);
    const [followPlayhead, setFollowPlayhead] = useState(true);
    const [showGhostNotes, setShowGhostNotes] = useState(true);
    const [ghostClips, setGhostClips] = useState<MidiClip[]>([]);
    const [selectedCCLane, setSelectedCCLane] = useState(-1);
    const [isFolded, setIsFolded] = useState(false);
    const [showArpeggiator, setShowArpeggiator] = useState(false);

    // Macro / Scripting
    const [isRecordingMacro, setIsRecordingMacro] = useState(false);
    const [macroBuffer, setMacroBuffer] = useState<{ command: string; args: any }[]>([]);
    const [savedMacros, setSavedMacros] = useState<{ name: string; steps: { command: string; args: any }[] }[]>([]);

    // Waveform Overlay
    const [tracks, setTracks] = useState<TrackInfo[]>([]);
    const [overlayTrackId, setOverlayTrackId] = useState<string>("");
    const [colorMode, setColorMode] = useState<ColorMode>('clip');
    const [groove, setGroove] = useState<string>('Straight');

    // ------------------------------------------------------------------------
    // ACTIONS & DATA LOADING
    // ------------------------------------------------------------------------
    const runAction = useCallback(async (command: string, args: any) => {
        if (isRecordingMacro) {
            setMacroBuffer(prev => [...prev, { command, args: JSON.parse(JSON.stringify(args)) }]);
        }

        const result = await invoke(command, args);

        if (clip && clip.pattern_id && ['add_midi_note', 'delete_midi_note', 'update_midi_note', 'quantize_notes'].includes(command)) {
            tracks.forEach((t, tIdx) => {
                t.midi_clips.forEach(c => {
                    if (c.pattern_id === clip.pattern_id && c.id !== clip.id) {
                        const siblingArgs = { ...args, trackIdx: tIdx, clipId: c.id };
                        invoke(command, siblingArgs).catch(console.error);
                    }
                });
            });
        }

        return result;
    }, [isRecordingMacro, clip, tracks]);

    const loadData = useCallback(async () => {
        try {
            const data = await invoke<MidiClip>('get_midi_clip_data', { trackIdx, clipId: currentClipId });
            setClip(data);

            const allClips = await invoke<MidiClip[]>('get_track_midi_clips', { trackIdx });
            const otherClips = allClips.filter(c => c.id !== currentClipId);
            setGhostClips(otherClips);
        } catch (error) {
            console.error('Failed to load MIDI clip data:', error);
        }
    }, [trackIdx, currentClipId]);

    const loadTracks = useCallback(async () => {
        try {
            const t = await invoke<TrackInfo[]>('get_tracks');
            setTracks(t);
        } catch (e) {
            console.error("Failed to load tracks", e);
        }
    }, []);

    const performCommand = useCallback(async (action: string, payload: any = {}) => {
        if (isRecordingMacro) {
            setMacroBuffer(prev => [...prev, { command: '__META__:' + action, args: payload }]);
        }

        if (action === 'DELETE_SELECTION') {
            const toDelete = Array.from(selection).sort((a, b) => b - a);
            for (const idx of toDelete) {
                await invoke('delete_midi_note', { trackIdx, clipId: currentClipId, noteIdx: idx });
            }
            setSelection(new Set());
            loadData();
        }
        else if (action === 'TRANSPOSE_SELECTION') {
            const { semitones } = payload;
            if (!clip) return;
            for (const idx of selection) {
                const note = clip.notes[idx];
                if (note) {
                    const newPitch = Math.min(127, Math.max(0, note.note + semitones));
                    await invoke('update_midi_note', {
                        trackIdx, clipId: currentClipId, noteIdx: idx, note: { ...note, note: newPitch }
                    });
                }
            }
            loadData();
        }
        else if (action === 'NUDGE_SELECTION') {
            const { delta } = payload;
            if (!clip) return;
            const nudgeSamples = (SAMPLES_PER_BEAT * 4 / snap) * delta;
            for (const idx of selection) {
                const note = clip.notes[idx];
                if (note) {
                    await invoke('update_midi_note', {
                        trackIdx, clipId: currentClipId, noteIdx: idx,
                        note: { ...note, start_sample: Math.max(0, note.start_sample + nudgeSamples) }
                    });
                }
            }
            loadData();
        }
        else if (action === 'HUMANIZE_SELECTION') {
            if (!clip) return;
            for (const idx of selection) {
                const note = clip.notes[idx];
                if (note) {
                    const randTime = Math.floor((Math.random() - 0.5) * 1000);
                    const randVel = Math.floor((Math.random() - 0.5) * 20);
                    await invoke('update_midi_note', {
                        trackIdx, clipId: currentClipId, noteIdx: idx,
                        note: { ...note, timing_random: randTime, velocity_random: randVel }
                    });
                }
            }
            loadData();
        }
        else if (action === 'QUANTIZE') {
            const { division } = payload;
            await invoke('quantize_notes', { trackIdx, clipId: currentClipId, division });
            loadData();
        }
    }, [isRecordingMacro, selection, clip, currentClipId, trackIdx, snap, loadData]);

    const snapToGrid = useCallback((samples: number): number => {
        const samplesPerSnap = SAMPLES_PER_BEAT * (4 / snap);
        return Math.round(samples / samplesPerSnap) * samplesPerSnap;
    }, [snap]);

    const applyGroove = useCallback(async (grooveName: string) => {
        setGroove(grooveName);
        const tmpl = {
            'Straight': { name: 'Straight', timing_offsets: new Array(16).fill(0), velocity_scale: new Array(16).fill(1) },
            'Swing 16-54': {
                name: 'Swing 16-54',
                timing_offsets: [0, 0.08, 0, 0.08, 0, 0.08, 0, 0.08, 0, 0.08, 0, 0.08, 0, 0.08, 0, 0.08],
                velocity_scale: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
            },
            'Swing 16-58': {
                name: 'Swing 16-58',
                timing_offsets: [0, 0.16, 0, 0.16, 0, 0.16, 0, 0.16, 0, 0.16, 0, 0.16, 0, 0.16, 0, 0.16],
                velocity_scale: [1, 0.9, 1, 0.9, 1, 0.9, 1, 0.9, 1, 0.9, 1, 0.9, 1, 0.9, 1, 0.9]
            },
            'Swing 16-62': {
                name: 'Swing 16-62',
                timing_offsets: [0, 0.24, 0, 0.24, 0, 0.24, 0, 0.24, 0, 0.24, 0, 0.24, 0, 0.24, 0, 0.24],
                velocity_scale: [1, 0.8, 1, 0.8, 1, 0.8, 1, 0.8, 1, 0.8, 1, 0.8, 1, 0.8, 1, 0.8]
            },
            'MPC 16-60': {
                name: 'MPC 16-60',
                timing_offsets: [0, 0.20, 0, 0.20, 0, 0.20, 0, 0.20, 0, 0.20, 0, 0.20, 0, 0.20, 0, 0.20],
                velocity_scale: [1.1, 0.8, 1.05, 0.85, 1.1, 0.8, 1.05, 0.85, 1.1, 0.8, 1.05, 0.85, 1.1, 0.8, 1.05, 0.85]
            }
        }[grooveName];

        if (tmpl) {
            try {
                await invoke('apply_groove_custom', {
                    trackIdx,
                    clipId: currentClipId,
                    template: tmpl
                });
                loadData();
            } catch (e) {
                console.error("Failed to apply groove", e);
            }
        }
    }, [trackIdx, currentClipId, loadData]);

    const startMidiLearn = useCallback(async (paramId: string) => {
        try {
            await invoke('start_midi_learn', { paramId });
            console.log("MIDI Learn started for", paramId);
        } catch (e) {
            console.error("Failed to start learn", e);
        }
    }, []);

    // ------------------------------------------------------------------------
    // KEYBOARD SHORTCUTS
    // ------------------------------------------------------------------------
    useEffect(() => {
        const handleKeyDown = async (e: KeyboardEvent) => {
            if (!clip) return;

            const selectedNotes = clip.notes.filter((_, i: number) => selection.has(i));

            if (e.ctrlKey && e.key.toLowerCase() === 'c') {
                setClipboard([...selectedNotes]);
            }

            if (e.ctrlKey && e.key.toLowerCase() === 'v') {
                if (clipboard.length > 0) {
                    const minStart = Math.min(...clipboard.map(n => n.start_sample));
                    const phRelative = playhead - clip.start_sample;
                    const snappedStart = snapToGrid(phRelative);

                    for (const note of clipboard) {
                        const newNote = {
                            ...note,
                            start_sample: snappedStart + (note.start_sample - minStart)
                        };
                        await runAction('add_midi_note', { trackIdx, clipId: currentClipId, note: newNote });
                    }
                    loadData();
                }
            }

            if (e.ctrlKey && e.key.toLowerCase() === 'd') {
                if (selection.size > 0) {
                    const indices = Array.from(selection);
                    await invoke('duplicate_midi_notes', { trackIdx, clipId: currentClipId, noteIndices: indices });
                    loadData();
                }
            }

            if (e.ctrlKey && e.key.toLowerCase() === 'z') {
                if (e.shiftKey) {
                    await invoke('redo');
                } else {
                    await invoke('undo');
                }
                loadData();
            }
            if (e.ctrlKey && e.key.toLowerCase() === 'y') {
                await invoke('redo');
                loadData();
            }

            if (e.ctrlKey && e.key.toLowerCase() === 'a') {
                e.preventDefault();
                if (clip && clip.notes) {
                    const allIndices = new Set(clip.notes.map((_, i) => i));
                    setSelection(allIndices);
                }
            }

            if (!e.ctrlKey && !e.shiftKey && !e.altKey && !e.metaKey) {
                if (e.key.toLowerCase() === 'b') setTool('brush');
                if (e.key.toLowerCase() === 'p') setTool('pencil');
                if (e.key.toLowerCase() === 'e') setTool('eraser');
                if (e.key === '1') setTool('select');
                if (e.key.toLowerCase() === 'q') {
                    await performCommand('QUANTIZE', { division: 'Sixteenth' });
                }
                if (e.key === ' ') {
                    e.preventDefault();
                    const playing = await invoke<boolean>('is_playing');
                    if (playing) {
                        await invoke('pause_audio');
                    } else {
                        await invoke('play_audio');
                    }
                }
            }

            if (e.key === 'Delete' || e.key === 'Backspace') {
                await performCommand('DELETE_SELECTION');
            }

            if (e.key.startsWith('Arrow') && selection.size > 0 && clip) {
                e.preventDefault();
                const pitchDelta = e.key === 'ArrowUp' ? 1 : e.key === 'ArrowDown' ? -1 : 0;
                const timeDelta = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0;
                const multiplier = e.shiftKey ? 12 : 1;

                if (pitchDelta !== 0) {
                    await performCommand('TRANSPOSE_SELECTION', { semitones: pitchDelta * multiplier });
                }

                if (timeDelta !== 0) {
                    await performCommand('NUDGE_SELECTION', { delta: timeDelta });
                }
            }
        };

        window.addEventListener('keydown', handleKeyDown);
        return () => window.removeEventListener('keydown', handleKeyDown);
    }, [clip, selection, clipboard, playhead, trackIdx, currentClipId, loadData, snapToGrid, runAction, performCommand]);

    // ------------------------------------------------------------------------
    // DATA INITIALIZATION & SYNCING
    // ------------------------------------------------------------------------
    useEffect(() => {
        loadData();
        loadTracks();
    }, [loadData, loadTracks]);

    useEffect(() => {
        const interval = setInterval(async () => {
            const ph = await invoke<number>('get_playhead');
            setPlayhead(ph);
        }, 50);
        return () => clearInterval(interval);
    }, []);

    // ------------------------------------------------------------------------
    // RENDER
    // ------------------------------------------------------------------------
    return (
        <div className="piano-roll-overlay" onClick={(e) => e.target === e.currentTarget && onClose()}>
            <div className="piano-roll-container">
                <PianoRollToolbar
                    trackIdx={trackIdx}
                    currentClipId={currentClipId}
                    setCurrentClipId={(id) => {
                        setCurrentClipId(id);
                        setSelection(new Set());
                    }}
                    clip={clip}
                    ghostClips={ghostClips}
                    showGhostNotes={showGhostNotes}
                    setShowGhostNotes={setShowGhostNotes}
                    overlayTrackId={overlayTrackId}
                    setOverlayTrackId={setOverlayTrackId}
                    tracks={tracks}
                    colorMode={colorMode}
                    setColorMode={setColorMode}
                    groove={groove}
                    applyGroove={applyGroove}
                    isRecordingMacro={isRecordingMacro}
                    setIsRecordingMacro={setIsRecordingMacro}
                    macroBuffer={macroBuffer}
                    setMacroBuffer={setMacroBuffer}
                    savedMacros={savedMacros}
                    setSavedMacros={setSavedMacros}
                    tool={tool}
                    setTool={setTool}
                    snap={snap}
                    setSnap={setSnap}
                    runAction={runAction}
                    performCommand={performCommand}
                    loadData={loadData}
                    followPlayhead={followPlayhead}
                    setFollowPlayhead={setFollowPlayhead}
                    isFolded={isFolded}
                    setIsFolded={setIsFolded}
                    setZoom={setZoom}
                    setShowArpeggiator={setShowArpeggiator}
                    startMidiLearn={startMidiLearn}
                    onClose={onClose}
                />

                <PianoRollGrid
                    trackIdx={trackIdx}
                    currentClipId={currentClipId}
                    clip={clip}
                    selection={selection}
                    setSelection={setSelection}
                    tool={tool}
                    snap={snap}
                    zoom={zoom}
                    setZoom={setZoom}
                    scroll={scroll}
                    setScroll={setScroll}
                    playhead={playhead}
                    showGhostNotes={showGhostNotes}
                    ghostClips={ghostClips}
                    selectedCCLane={selectedCCLane}
                    setSelectedCCLane={setSelectedCCLane}
                    isFolded={isFolded}
                    tracks={tracks}
                    overlayTrackId={overlayTrackId}
                    colorMode={colorMode}
                    loadData={loadData}
                    runAction={runAction}
                    performCommand={performCommand}
                    snapToGrid={snapToGrid}
                    setHoverInfo={setHoverInfo}
                />

                <div className="piano-roll-status">
                    <div className="status-item">PITCH: <span>{hoverInfo ? getNoteName(hoverInfo.note) : '--'}</span></div>
                    <div className="status-item">TIME: <span>{hoverInfo ? hoverInfo.time.toFixed(3) : '0.000'}</span></div>
                    <div className="status-item">NOTES: <span>{clip?.notes.length || 0}</span></div>
                    <div className="status-item" style={{ marginLeft: 'auto' }}>VIBE COMPOSER SUITE v2.5</div>
                </div>

                {selection.size > 0 && clip && (
                    <div className="note-properties-panel">
                        <h4>PROPERTIES ({selection.size})</h4>
                        <div className="prop-row">
                            <label>Velocity</label>
                            <input type="range" min="0" max="127"
                                data-testid="note-velocity-slider"
                                onChange={(e) => {
                                    if (!clip) return;
                                    const val = parseInt(e.target.value);
                                    selection.forEach((idx: number) => {
                                        const note = clip.notes[idx];
                                        if (note) {
                                            runAction('update_midi_note', { trackIdx, clipId: currentClipId, noteIdx: idx, note: { ...note, velocity: Math.floor(val * 33818640) } });
                                        }
                                    });
                                    loadData();
                                }}
                            />
                        </div>
                        <div className="prop-row">
                            <label>Probability</label>
                            <input type="range" min="0" max="100"
                                data-testid="note-probability-slider"
                                onChange={(e) => {
                                    if (!clip) return;
                                    const val = parseInt(e.target.value) / 100;
                                    selection.forEach((idx: number) => {
                                        const note = clip.notes[idx];
                                        if (note) {
                                            runAction('update_midi_note', { trackIdx, clipId: currentClipId, noteIdx: idx, note: { ...note, probability: val } });
                                        }
                                    });
                                    loadData();
                                }}
                            />
                        </div>
                        <div className="prop-row">
                            <label>Humanize</label>
                            <button onClick={() => {
                                performCommand('HUMANIZE_SELECTION');
                             }}>APPLY</button>
                        </div>

                        <div className="prop-row">
                            <label>Channel</label>
                            <select
                                style={{ flex: 1, background: '#111', color: '#eee', border: '1px solid #333', borderRadius: '4px', padding: '2px' }}
                                value={clip && selection.size > 0 ? (clip.notes[Array.from(selection)[0]]?.channel || 0) : 0}
                                onChange={(e) => {
                                    if (!clip) return;
                                    const val = parseInt(e.target.value);
                                    selection.forEach((idx: number) => {
                                        const note = clip.notes[idx];
                                        if (note) {
                                            runAction('update_midi_note', {
                                                trackIdx,
                                                clipId: currentClipId,
                                                noteIdx: idx,
                                                note: { ...note, channel: val }
                                            });
                                        }
                                    });
                                    loadData();
                                }}
                            >
                                {Array.from({ length: 16 }, (_, i: number) => (
                                    <option key={i} value={i} style={{ color: CHANNEL_COLORS[i] }}>Channel {i + 1}</option>
                                ))}
                            </select>
                        </div>

                        <div className="prop-divider" style={{ marginTop: '10px', paddingTop: '10px', borderTop: '1px solid #333' }}>MPE EXPRESSION</div>

                        <div className="prop-row">
                            <label>Pressure</label>
                            <input type="range" min="0" max="127"
                                defaultValue={clip && selection.size > 0 ? (clip.notes[Array.from(selection)[0]]?.pressure || 0) : 0}
                                onChange={(e) => {
                                    if (!clip) return;
                                    const val = parseInt(e.target.value);
                                    selection.forEach((idx: number) => {
                                        const note = clip.notes[idx];
                                        if (note) {
                                            runAction('update_midi_note', {
                                                trackIdx,
                                                clipId: currentClipId,
                                                noteIdx: idx,
                                                note: { ...note, pressure: val }
                                            });
                                        }
                                    });
                                    loadData();
                                }}
                            />
                        </div>

                        <div className="prop-row">
                            <label>Timbre</label>
                            <input type="range" min="0" max="127"
                                defaultValue={clip && selection.size > 0 ? (clip.notes[Array.from(selection)[0]]?.timbre || 64) : 64}
                                onChange={(e) => {
                                    if (!clip) return;
                                    const val = parseInt(e.target.value);
                                    selection.forEach((idx: number) => {
                                        const note = clip.notes[idx];
                                        if (note) {
                                            runAction('update_midi_note', {
                                                trackIdx,
                                                clipId: currentClipId,
                                                noteIdx: idx,
                                                note: { ...note, timbre: val }
                                            });
                                        }
                                    });
                                    loadData();
                                }}
                            />
                        </div>

                        <div className="prop-row">
                            <label>Pitch Bend</label>
                            <input type="range" min="-8192" max="8192"
                                defaultValue={clip && selection.size > 0 ? (clip.notes[Array.from(selection)[0]]?.pitch_bend || 0) : 0}
                                onChange={(e) => {
                                    if (!clip) return;
                                    const val = parseInt(e.target.value);
                                    selection.forEach((idx: number) => {
                                        const note = clip.notes[idx];
                                        if (note) {
                                            runAction('update_midi_note', {
                                                trackIdx,
                                                clipId: currentClipId,
                                                noteIdx: idx,
                                                note: { ...note, pitch_bend: val }
                                            });
                                        }
                                    });
                                    loadData();
                                }}
                            />
                        </div>
                    </div>
                )}
            </div>

            <ArpeggiatorModal
                show={showArpeggiator}
                onClose={() => setShowArpeggiator(false)}
                clip={clip}
                selection={selection}
                trackIdx={trackIdx}
                clipId={currentClipId}
                loadData={loadData}
                setSelection={setSelection}
                SAMPLES_PER_BEAT={SAMPLES_PER_BEAT}
            />
        </div>
    );
}
