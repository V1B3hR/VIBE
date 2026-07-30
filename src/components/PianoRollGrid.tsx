import React, { useEffect, useRef, useState, useCallback, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
    MidiClip,
    MidiNote,
    TrackInfo,
    Tool,
    DragState,
    DragType,
    ColorMode,
    BLACK_KEYS,
    CHANNEL_COLORS,
    SCALE_INTERVALS,
    ScaleType,
    getNoteName
} from './PianoRollTypes';

interface PianoRollGridProps {
    trackIdx: number;
    currentClipId: string;
    clip: MidiClip | null;
    selection: Set<number>;
    setSelection: React.Dispatch<React.SetStateAction<Set<number>>>;
    tool: Tool;
    snap: number;
    zoom: { x: number; y: number };
    setZoom: React.Dispatch<React.SetStateAction<{ x: number; y: number }>>;
    scroll: { x: number; y: number };
    setScroll: React.Dispatch<React.SetStateAction<{ x: number; y: number }>>;
    playhead: number;
    showGhostNotes: boolean;
    ghostClips: MidiClip[];
    selectedCCLane: number;
    setSelectedCCLane: React.Dispatch<React.SetStateAction<number>>;
    isFolded: boolean;
    tracks: TrackInfo[];
    overlayTrackId: string;
    colorMode: ColorMode;
    loadData: () => void;
    runAction: (command: string, args: any) => Promise<any>;
    performCommand: (action: string, payload?: any) => Promise<void>;
    snapToGrid: (samples: number) => number;
    setHoverInfo: (info: { note: number; time: number } | null) => void;
}

const SIDEBAR_WIDTH = 60;
const SAMPLE_RATE = 44100;
const BPM = 120;

export function PianoRollGrid({
    trackIdx,
    currentClipId,
    clip,
    selection,
    setSelection,
    tool,
    snap,
    zoom,
    setZoom,
    scroll,
    setScroll,
    playhead,
    showGhostNotes,
    ghostClips,
    selectedCCLane,
    setSelectedCCLane,
    isFolded,
    tracks,
    overlayTrackId,
    colorMode,
    loadData,
    runAction,
    performCommand,
    snapToGrid,
    setHoverInfo
}: PianoRollGridProps) {
    const mainCanvasRef = useRef<HTMLCanvasElement>(null);
    const bgCanvasRef = useRef<HTMLCanvasElement>(null);
    const velocityCanvasRef = useRef<HTMLCanvasElement>(null);
    const containerRef = useRef<HTMLDivElement>(null);
    const lastBgState = useRef<string>("");

    const [isDragging, setIsDragging] = useState(false);
    const [dragStart, setDragStart] = useState<DragState>({
        x: 0,
        y: 0,
        noteIdx: -1,
        type: 'move',
        originalNote: null
    });
    const [lastPainted, setLastPainted] = useState<{ time: number; pitch: number } | null>(null);
    const [mousePos, setMousePos] = useState({ x: 0, y: 0 });

    // Derived values
    const KEY_HEIGHT = 20 * zoom.y;
    const PIXELS_PER_BEAT = 100 * zoom.x;
    const SAMPLES_PER_BEAT = (SAMPLE_RATE * 60) / BPM;
    const PIXELS_PER_SAMPLE = PIXELS_PER_BEAT / SAMPLES_PER_BEAT;

    const foldedNoteMap = useMemo(() => {
        if (!isFolded || !clip) return null;
        const notes = new Set<number>();
        clip.notes.forEach(n => notes.add(n.note));
        return Array.from(notes).sort((a, b) => b - a);
    }, [isFolded, clip]);

    const getNoteColor = useCallback((note: MidiNote, clipColor: string, isSelected: boolean) => {
        if (isSelected) return '#fff';
        switch (colorMode) {
            case 'clip': return clipColor || '#4a9eff';
            case 'channel': return CHANNEL_COLORS[note.channel % 16] || '#00cc88';
            case 'velocity':
                const maxVel = 4294967295; // 32-bit max
                const hue = 240 - (note.velocity / maxVel) * 240;
                return `hsl(${hue}, 70%, 50%)`;
            case 'pitch':
                return `hsl(${(note.note % 12) * 30}, 80%, 60%)`;
            default: return '#4a9eff';
        }
    }, [colorMode]);

    const findNoteAt = useCallback((x: number, y: number): number => {
        if (!clip) return -1;

        const time = (x - SIDEBAR_WIDTH + scroll.x) / PIXELS_PER_SAMPLE;
        const steps = clip.tuning_steps || 12;
        const totalNotes = Math.ceil(10.7 * steps);
        const pitch = (totalNotes - 1) - Math.floor((y + scroll.y) / KEY_HEIGHT);

        return clip.notes.findIndex((note: MidiNote) => {
            if (note.note !== pitch) return false;
            const noteStart = note.start_sample;
            const noteEnd = noteStart + note.length_samples;
            return time >= noteStart && time <= noteEnd;
        });
    }, [clip, scroll.x, scroll.y, PIXELS_PER_SAMPLE, KEY_HEIGHT]);

    const draw = useCallback(() => {
        const canvas = mainCanvasRef.current;
        const bgCanvas = bgCanvasRef.current;
        if (!canvas || !bgCanvas || !clip) return;

        const ctx = canvas.getContext('2d');
        const bgCtx = bgCanvas.getContext('2d');
        if (!ctx || !bgCtx) return;

        if (canvas.width !== canvas.offsetWidth || canvas.height !== canvas.offsetHeight) {
            canvas.width = canvas.offsetWidth;
            canvas.height = canvas.offsetHeight;
            bgCanvas.width = bgCanvas.offsetWidth;
            bgCanvas.height = bgCanvas.offsetHeight;
            lastBgState.current = "";
        }

        const width = canvas.width;
        const height = canvas.height;

        // 1. STATIC BACKGROUND LAYER (Grid, Sidebar, Waveform)
        const currentBgMeta = `${zoom.x}-${zoom.y}-${scroll.x}-${scroll.y}-${clip.tuning_steps}-${clip.time_signature_num}-${clip.time_signature_den}-${overlayTrackId}-${isFolded}-${foldedNoteMap?.length}`;
        if (currentBgMeta !== lastBgState.current) {
            lastBgState.current = currentBgMeta;

            bgCtx.fillStyle = '#1a1a1a';
            bgCtx.fillRect(0, 0, width, height);

            // Waveform Overlay (Background)
            if (overlayTrackId) {
                const track = tracks.find(t => t.id === overlayTrackId);
                if (track && track.clips) {
                    bgCtx.globalAlpha = 0.2;
                    bgCtx.fillStyle = '#4a9eff';
                    track.clips.forEach(audioClip => {
                        const relativeStart = audioClip.start_sample - clip.start_sample;
                        const clipX = SIDEBAR_WIDTH + relativeStart * PIXELS_PER_SAMPLE - scroll.x;
                        const clipW = audioClip.length_samples * PIXELS_PER_SAMPLE;

                        if (clipX + clipW > SIDEBAR_WIDTH && clipX < width) {
                            const peaks = audioClip.peaks && audioClip.peaks[0];
                            if (peaks && peaks.length > 0) {
                                const bestPeaks = (clipW > 2000 && audioClip.peaks[1]) ? audioClip.peaks[1] : peaks;
                                const peakStep = clipW / bestPeaks.length;
                                bgCtx.beginPath();
                                bestPeaks.forEach((p, i) => {
                                    const px = clipX + i * peakStep;
                                    if (px >= SIDEBAR_WIDTH - peakStep && px < width) {
                                        const h = height * p;
                                        const py = (height - h) / 2;
                                        bgCtx.rect(px, py, Math.max(1, peakStep), h);
                                    }
                                });
                                bgCtx.fill();
                            }
                        }
                    });
                    bgCtx.globalAlpha = 1.0;
                }
            }

            // Grid Lines (Vertical)
            bgCtx.strokeStyle = '#333';
            bgCtx.lineWidth = 1;
            const beatsVisible = Math.ceil(width / PIXELS_PER_BEAT) + 2;
            const startBeat = Math.floor(scroll.x / PIXELS_PER_BEAT);
            bgCtx.beginPath();
            for (let i = 0; i < beatsVisible; i++) {
                const beat = startBeat + i;
                const x = SIDEBAR_WIDTH + beat * PIXELS_PER_BEAT - scroll.x;
                if (x >= SIDEBAR_WIDTH) {
                    bgCtx.moveTo(x, 0);
                    bgCtx.lineTo(x, height);
                }
            }
            bgCtx.stroke();

            // Bar Lines
            const tsNum = clip.time_signature_num || 4;
            const tsDen = clip.time_signature_den || 4;
            const barLen = tsNum * (4 / tsDen);
            bgCtx.strokeStyle = '#444';
            bgCtx.lineWidth = 2;
            bgCtx.beginPath();
            const startBar = Math.floor(startBeat / barLen);
            const endBar = startBar + Math.ceil(beatsVisible / barLen) + 1;
            for (let b = startBar; b <= endBar; b++) {
                const barPos = b * barLen;
                const x = SIDEBAR_WIDTH + barPos * PIXELS_PER_BEAT - scroll.x;
                if (x >= SIDEBAR_WIDTH && x < width) {
                    bgCtx.moveTo(x, 0);
                    bgCtx.lineTo(x, height);
                }
            }
            bgCtx.stroke();

            // Piano Keys & Scale Highlighting (Vertical)
            const steps = clip.tuning_steps || 12;
            const totalNotes = Math.ceil(10.7 * steps);
            const visibleNoteCount = foldedNoteMap ? foldedNoteMap.length : totalNotes;
            const startYIndex = Math.floor(scroll.y / KEY_HEIGHT);
            const endYIndex = Math.ceil((scroll.y + height) / KEY_HEIGHT) + 1;

            for (let i = startYIndex; i < Math.min(endYIndex, visibleNoteCount); i++) {
                const note = foldedNoteMap ? foldedNoteMap[i] : (totalNotes - 1) - i;
                const y = i * KEY_HEIGHT - scroll.y;
                if (note < 0) break;

                // Scale highlight
                if (steps === 12 && clip.scale) {
                    const noteInScale = SCALE_INTERVALS[clip.scale.type as ScaleType]?.includes((note - clip.scale.root) % 12);
                    if (noteInScale) {
                        bgCtx.fillStyle = '#222';
                        bgCtx.fillRect(SIDEBAR_WIDTH, y, width - SIDEBAR_WIDTH, KEY_HEIGHT);
                    }
                }

                // Key separator
                bgCtx.strokeStyle = '#333';
                bgCtx.beginPath();
                bgCtx.moveTo(SIDEBAR_WIDTH, y);
                bgCtx.lineTo(width, y);
                bgCtx.stroke();

                // Sidebar Key
                if (steps === 12) {
                    const isBlackKey = BLACK_KEYS.includes(note % 12);
                    bgCtx.fillStyle = isBlackKey ? '#000' : '#fff';
                    bgCtx.fillRect(0, y, SIDEBAR_WIDTH, KEY_HEIGHT);
                    if (isFolded || note % 12 === 0) {
                        bgCtx.fillStyle = isBlackKey ? '#fff' : '#000';
                        bgCtx.font = '9px monospace';
                        bgCtx.fillText(getNoteName(note), 5, y + KEY_HEIGHT / 2 + 3);
                    }
                } else {
                    const isOctave = note % steps === 0;
                    bgCtx.fillStyle = isOctave ? '#444' : (note % 2 === 0 ? '#333' : '#222');
                    bgCtx.fillRect(0, y, SIDEBAR_WIDTH, KEY_HEIGHT);
                    if (isOctave) {
                        bgCtx.fillStyle = '#aaa';
                        bgCtx.font = '9px monospace';
                        bgCtx.fillText(`O${Math.floor(note / steps)}`, 5, y + KEY_HEIGHT / 2 + 3);
                    }
                }
            }
        }

        // 2. DYNAMIC FOREGROUND LAYER (Notes, Selection, Playhead)
        ctx.clearRect(0, 0, width, height);

        // Ghost Notes
        if (showGhostNotes && ghostClips.length > 0) {
            ctx.globalAlpha = 0.2;
            ctx.fillStyle = '#888';
            ghostClips.forEach(gClip => {
                gClip.notes.forEach(note => {
                    const x = SIDEBAR_WIDTH + note.start_sample * PIXELS_PER_SAMPLE - scroll.x;
                    const w = note.length_samples * PIXELS_PER_SAMPLE;
                    if (x + w < SIDEBAR_WIDTH || x > width) return;

                    const y = (127 - note.note) * KEY_HEIGHT - scroll.y;
                    if (y + KEY_HEIGHT < 0 || y > height) return;

                    ctx.fillRect(x, y, w, KEY_HEIGHT - 1);
                });
            });
            ctx.globalAlpha = 1.0;
        }

        // Clip Notes
        if (clip.notes) {
            const steps = clip.tuning_steps || 12;
            const totalNotes = Math.ceil(10.7 * steps);

            clip.notes.forEach((note, idx) => {
                const x = SIDEBAR_WIDTH + note.start_sample * PIXELS_PER_SAMPLE - scroll.x;
                const w = note.length_samples * PIXELS_PER_SAMPLE;
                if (x + w < SIDEBAR_WIDTH || x > width) return;

                const visibleNoteCount = foldedNoteMap ? foldedNoteMap.length : totalNotes;
                let noteYIndex = -1;
                if (foldedNoteMap) {
                    noteYIndex = foldedNoteMap.indexOf(note.note);
                } else {
                    noteYIndex = totalNotes - 1 - note.note;
                }

                if (noteYIndex === -1) return;
                const y = noteYIndex * KEY_HEIGHT - scroll.y;
                if (y + KEY_HEIGHT < 0 || y > height) return;

                const isSelected = selection.has(idx);
                ctx.fillStyle = getNoteColor(note, clip.color || '#4c7cff', isSelected);
                ctx.fillRect(x, y, w, KEY_HEIGHT - 1);

                // Velocity line
                const velocityH = (note.velocity / 127) * (KEY_HEIGHT - 1);
                ctx.fillStyle = 'rgba(255, 255, 255, 0.2)';
                ctx.fillRect(x, y + KEY_HEIGHT - 1 - velocityH, w, velocityH);

                // Border
                ctx.strokeStyle = isSelected ? '#5599ff' : 'rgba(255,255,255,0.3)';
                ctx.lineWidth = isSelected ? 2 : 1;
                ctx.strokeRect(x, y, w, KEY_HEIGHT - 1);

                // MPE visualizations (Pitch/Pressure/Timbre)
                if (note.pitch_bend && note.pitch_bend !== 0) {
                    const bendY = y + (KEY_HEIGHT / 2) + (note.pitch_bend / 8192) * KEY_HEIGHT;
                    ctx.strokeStyle = '#ff6600';
                    ctx.beginPath();
                    ctx.moveTo(x, y + KEY_HEIGHT / 2);
                    ctx.lineTo(x + w, bendY);
                    ctx.stroke();
                }

                if ((note.pressure ?? 0) > 0) {
                    ctx.fillStyle = `rgba(255, 100, 255, ${(note.pressure ?? 0) / 200})`;
                    ctx.fillRect(x, y, 4, KEY_HEIGHT - 1);
                }

                if ((note.timbre ?? 64) !== 64) {
                    const timbreShift = ((note.timbre ?? 64) - 64) / 64; // -1 to 1
                    ctx.fillStyle = timbreShift > 0 ? '#ffcc00' : '#00ffff';
                    ctx.globalAlpha = Math.abs(timbreShift) * 0.5;
                    ctx.fillRect(x + w - 4, y, 4, KEY_HEIGHT - 1);
                    ctx.globalAlpha = 1.0;
                }
            });
        }

        // Playhead
        const phRelative = playhead - clip.start_sample;
        const phX = SIDEBAR_WIDTH + phRelative * PIXELS_PER_SAMPLE - scroll.x;
        if (phX >= SIDEBAR_WIDTH && phX < width) {
            ctx.strokeStyle = '#ff3b3b';
            ctx.lineWidth = 2;
            ctx.beginPath();
            ctx.moveTo(phX, 0);
            ctx.lineTo(phX, height);
            ctx.stroke();
        }

        // Lasso Selection
        if (isDragging && dragStart.type === 'lasso') {
            const dragW = mousePos.x - dragStart.x;
            const dragH = mousePos.y - dragStart.y;
            ctx.strokeStyle = '#0078d4';
            ctx.lineWidth = 1;
            ctx.setLineDash([5, 5]);
            ctx.strokeRect(dragStart.x, dragStart.y, dragW, dragH);
            ctx.fillStyle = 'rgba(0, 120, 212, 0.1)';
            ctx.fillRect(dragStart.x, dragStart.y, dragW, dragH);
            ctx.setLineDash([]);
        }
    }, [clip, zoom, selection, scroll, playhead, showGhostNotes, ghostClips, overlayTrackId, tracks, KEY_HEIGHT, PIXELS_PER_BEAT, PIXELS_PER_SAMPLE, isDragging, dragStart, mousePos, getNoteColor]);

    const drawVelocityLane = useCallback(() => {
        const canvas = velocityCanvasRef.current;
        if (!canvas || !clip) return;

        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        if (canvas.width !== canvas.offsetWidth || canvas.height !== canvas.offsetHeight) {
            canvas.width = canvas.offsetWidth;
            canvas.height = canvas.offsetHeight;
        }

        const width = canvas.width;
        const height = canvas.height;

        ctx.fillStyle = '#0a0a0a';
        ctx.fillRect(0, 0, width, height);

        ctx.strokeStyle = '#222';
        const beatsVisible = Math.ceil(width / PIXELS_PER_BEAT) + 2;
        const startBeat = Math.floor(scroll.x / PIXELS_PER_BEAT);
        ctx.beginPath();
        for (let i = 0; i < beatsVisible; i++) {
            const beat = startBeat + i;
            const x = beat * PIXELS_PER_BEAT - scroll.x;
            ctx.moveTo(x, 0);
            ctx.lineTo(x, height);
        }
        ctx.stroke();

        if (selectedCCLane === -1) {
            ctx.fillStyle = '#00cc88';
            clip.notes.forEach((note, idx) => {
                const x = note.start_sample * PIXELS_PER_SAMPLE - scroll.x;
                const w = 4;
                if (x + w < 0 || x > width) return;

                const isSelected = selection.has(idx);
                const barH = (note.velocity / 127) * height;
                ctx.fillStyle = isSelected ? '#5599ff' : '#00cc88';
                ctx.fillRect(x, height - barH, w, barH);
            });
        }
    }, [clip, scroll.x, PIXELS_PER_BEAT, PIXELS_PER_SAMPLE, selection, selectedCCLane]);

    const handleMouseDown = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
        if (!clip) return;

        const canvas = mainCanvasRef.current;
        if (!canvas) return;

        const rect = canvas.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;

        if (x < SIDEBAR_WIDTH) return;

        if (tool === 'pencil' || tool === 'brush') {
            const time = snapToGrid((x - SIDEBAR_WIDTH + scroll.x) / PIXELS_PER_SAMPLE);
            const steps = clip.tuning_steps || 12;
            const totalNotes = Math.ceil(10.7 * steps);
            const pitch = (totalNotes - 1) - Math.floor((y + scroll.y) / KEY_HEIGHT);
            const snappedLength = snapToGrid(SAMPLES_PER_BEAT);

            runAction('add_midi_note', {
                trackIdx,
                clipId: currentClipId,
                note: {
                    start_sample: time,
                    length_samples: snappedLength,
                    note: pitch,
                    velocity: 100 * 33818640,
                    channel: 0,
                    probability: 1.0,
                    velocity_random: 0,
                    timing_random: 0
                }
            }).then(loadData);

            if (tool === 'brush') {
                setIsDragging(true);
                setDragStart({ x, y, noteIdx: -1, type: 'paint', originalNote: null });
                setLastPainted({ time, pitch });
            }
        } else if (tool === 'eraser') {
            const noteIdx = findNoteAt(x, y);
            if (noteIdx >= 0) {
                runAction('delete_midi_note', { trackIdx, clipId: currentClipId, noteIdx }).then(loadData);
            }
        } else if (tool === 'select') {
            const noteIdx = findNoteAt(x, y);

            if (noteIdx >= 0) {
                if (!e.shiftKey && !selection.has(noteIdx)) {
                    setSelection(new Set([noteIdx]));
                }
                setIsDragging(true);
                setDragStart({ x, y, noteIdx, type: 'move', originalNote: clip.notes[noteIdx] });
            } else {
                setSelection(new Set());
                setIsDragging(true);
                setDragStart({ x, y, noteIdx: -1, type: 'lasso', originalNote: null });
            }
        }
    }, [clip, tool, snapToGrid, scroll.x, scroll.y, PIXELS_PER_SAMPLE, KEY_HEIGHT, SAMPLES_PER_BEAT, trackIdx, currentClipId, loadData, findNoteAt, selection, setSelection, runAction]);

    const handleMouseMove = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
        const canvas = mainCanvasRef.current;
        if (!canvas || !clip) return;

        const rect = canvas.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;
        setMousePos({ x, y });

        if (x < SIDEBAR_WIDTH) return;

        const time = (x - SIDEBAR_WIDTH + scroll.x) / PIXELS_PER_SAMPLE / SAMPLE_RATE;
        const steps = clip.tuning_steps || 12;
        const totalNotes = Math.ceil(10.7 * steps);
        const pitch = (totalNotes - 1) - Math.floor((y + scroll.y) / KEY_HEIGHT);
        setHoverInfo({ note: pitch, time });

        if (!isDragging || !clip) return;

        if (tool === 'brush' && dragStart.type === 'paint') {
            const time = snapToGrid((x - SIDEBAR_WIDTH + scroll.x) / PIXELS_PER_SAMPLE);

            if (!lastPainted || lastPainted.time !== time || lastPainted.pitch !== pitch) {
                const exists = clip.notes.some(n =>
                    n.note === pitch &&
                    Math.abs(n.start_sample - time) < snapToGrid(SAMPLES_PER_BEAT) / 2
                );

                if (!exists) {
                    const snappedLength = snapToGrid(SAMPLES_PER_BEAT);
                    runAction('add_midi_note', {
                        trackIdx,
                        clipId: currentClipId,
                        note: {
                            start_sample: time,
                            length_samples: snappedLength,
                            note: pitch,
                            velocity: 100 * 33818640,
                            channel: 0,
                            velocity_random: 0,
                            timing_random: 0
                        }
                    }).then(loadData);
                    setLastPainted({ time, pitch });
                }
            }
        } else if (dragStart.type === 'move' && dragStart.noteIdx >= 0 && dragStart.originalNote) {
            const dx = x - dragStart.x;
            const dy = y - dragStart.y;
            const timeDelta = dx / PIXELS_PER_SAMPLE;
            const pitchDelta = -Math.round(dy / KEY_HEIGHT);

            const newTime = snapToGrid(dragStart.originalNote.start_sample + timeDelta);
            const newPitch = Math.max(0, Math.min(127, dragStart.originalNote.note + pitchDelta));

            // To avoid flickering during drag, we update local clip representation
            // but update on backend on MouseUp.
            const updatedNotes = [...clip.notes];
            updatedNotes[dragStart.noteIdx] = {
                ...dragStart.originalNote,
                start_sample: newTime,
                note: newPitch
            };
            clip.notes = updatedNotes;
        }
    }, [isDragging, clip, tool, dragStart, snapToGrid, scroll.x, scroll.y, PIXELS_PER_SAMPLE, KEY_HEIGHT, SAMPLES_PER_BEAT, lastPainted, trackIdx, currentClipId, loadData, runAction, setHoverInfo]);

    const handleMouseUp = useCallback(() => {
        setIsDragging(false);
        if (isDragging && dragStart.type === 'move' && dragStart.noteIdx >= 0 && clip) {
            const note = clip.notes[dragStart.noteIdx];
            runAction('update_midi_note', {
                trackIdx,
                clipId: currentClipId,
                noteIdx: dragStart.noteIdx,
                note
            }).then(loadData);
        }
    }, [isDragging, dragStart, clip, trackIdx, currentClipId, loadData, runAction]);

    // Animation Loop
    useEffect(() => {
        let requestId: number;
        const render = () => {
            draw();
            drawVelocityLane();
            requestId = requestAnimationFrame(render);
        };
        requestId = requestAnimationFrame(render);
        return () => cancelAnimationFrame(requestId);
    }, [draw, drawVelocityLane]);

    return (
        <div className="piano-roll-main">
            <div
                className="piano-roll-canvas-container"
                ref={containerRef}
                onWheel={(e) => {
                    if (e.ctrlKey) {
                        setZoom(z => ({ ...z, x: Math.max(0.1, z.x - e.deltaY * 0.001) }));
                    } else if (e.altKey) {
                        setZoom(z => ({ ...z, y: Math.max(0.2, z.y - e.deltaY * 0.001) }));
                    } else {
                        setScroll(s => ({
                            x: Math.max(0, s.x + e.deltaX + (e.shiftKey ? e.deltaY : 0)),
                            y: Math.max(0, Math.min(128 * KEY_HEIGHT - 300, s.y + (e.shiftKey ? 0 : e.deltaY)))
                        }));
                    }
                }}
            >
                <canvas
                    ref={bgCanvasRef}
                    className="canvas-bg"
                    style={{ pointerEvents: 'none' }}
                />
                <canvas
                    ref={mainCanvasRef}
                    className="canvas-main"
                    onMouseDown={handleMouseDown}
                    onMouseMove={handleMouseMove}
                    onMouseUp={handleMouseUp}
                    onMouseLeave={() => setHoverInfo(null)}
                    data-testid="piano-roll-canvas"
                />
            </div>

            <div className="piano-roll-cc-lanes">
                <div className="cc-lane-sidebar">
                    <select
                        className="toolbar-select"
                        style={{ fontSize: '9px', width: '50px' }}
                        value={selectedCCLane}
                        onChange={(e) => setSelectedCCLane(parseInt(e.target.value))}
                    >
                        <option value={-1}>VEL</option>
                        <option value={1}>MOD</option>
                        <option value={11}>EXPR</option>
                        <option value={74}>TIMBRE</option>
                        <option value={64}>SUSTAIN</option>
                    </select>
                </div>
                <canvas
                    className="cc-lane-canvas"
                    ref={velocityCanvasRef}
                    onMouseDown={(e) => {
                        if (!clip) return;
                        const rect = e.currentTarget.getBoundingClientRect();
                        const x = e.clientX - rect.left;
                        const y = e.clientY - rect.top;

                        const noteIdx = clip.notes.findIndex(n => {
                            const nx = n.start_sample * PIXELS_PER_SAMPLE - scroll.x;
                            return x >= nx && x <= nx + 4;
                        });

                        if (noteIdx >= 0) {
                            setIsDragging(true);
                            setDragStart({ x: e.clientX, y: e.clientY, noteIdx, type: 'velocity', originalNote: clip.notes[noteIdx] });

                            const newVel = Math.max(0, Math.min(127, Math.floor((1 - y / rect.height) * 127)));
                            const scaledVel = Math.floor(newVel * 33818640);
                            runAction('update_midi_note', {
                                trackIdx,
                                clipId: currentClipId,
                                noteIdx,
                                note: { ...clip.notes[noteIdx], velocity: scaledVel }
                            }).then(loadData);
                        }
                    }}
                    onMouseMove={(e) => {
                        if (!isDragging || dragStart.type !== 'velocity' || !clip) return;
                        const rect = e.currentTarget.getBoundingClientRect();
                        const x = e.clientX - rect.left;
                        const y = e.clientY - rect.top;

                        const noteIdx = clip.notes.findIndex(n => {
                            const nx = n.start_sample * PIXELS_PER_SAMPLE - scroll.x;
                            return x >= nx && x <= nx + 4;
                        });

                        if (noteIdx >= 0) {
                            const newVel = Math.max(0, Math.min(127, Math.floor((1 - y / rect.height) * 127)));
                            const scaledVel = Math.floor(newVel * 33818640);
                            if (clip.notes[noteIdx].velocity !== scaledVel) {
                                runAction('update_midi_note', {
                                    trackIdx,
                                    clipId: currentClipId,
                                    noteIdx,
                                    note: { ...clip.notes[noteIdx], velocity: scaledVel }
                                }).then(loadData);
                            }
                        }
                    }}
                />
            </div>
        </div>
    );
}
