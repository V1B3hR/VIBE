import * as React from 'react';
import { useState, useEffect, useRef, useMemo, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { SampleEditor } from "./SampleEditor";
import { PianoRoll } from "./PianoRoll";
import "./Timeline.css";

import { DndContext, closestCenter, KeyboardSensor, PointerSensor, useSensor, useSensors, DragEndEvent } from '@dnd-kit/core';
import { SortableContext, sortableKeyboardCoordinates, verticalListSortingStrategy, useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';

import { TimelineRuler } from "./TimelineRuler";
import { TimelineContextMenu } from "./TimelineContextMenu";
import { TrackContextMenu } from "./TrackContextMenu";
import { ClipContextMenu } from "./ClipContextMenu";
import { OverviewBar } from "./OverviewBar";
import { TimelineToolbar } from "./TimelineToolbar";
import { TrackLane } from "./TrackLane";
import { useTimelineLogic } from "../hooks/useTimelineLogic";
import { Clip } from "../types/timeline";

const SortableTrackItem = ({ track, top, depth, children }: { track: any, top: number, depth: number, children: (dragProps: any) => React.ReactNode }) => {
    const {
        attributes,
        listeners,
        setNodeRef,
        setActivatorNodeRef,
        transform,
        transition,
        isDragging,
    } = useSortable({ id: track.id });

    const style = {
        position: 'absolute' as const,
        top: top,
        left: 0,
        right: 0,
        width: '100%',
        paddingLeft: `${depth * 20}px`,
        transform: CSS.Translate.toString(transform),
        transition,
        zIndex: isDragging ? 100 : 1,
        opacity: isDragging ? 0.8 : 1,
    };

    return (
        <div ref={setNodeRef} style={style} className={depth > 0 ? 'track-indent' : undefined}>
            {children({ dragRef: setActivatorNodeRef, dragAttributes: attributes, dragListeners: listeners })}
        </div>
    );
};

export const Timeline = () => {
    const {
        tracks, setTracks,
        playhead,
        bpm,
        snap, setSnap,
        selectedClips, setSelectedClips,
        loopStart, setLoopStart,
        loopEnd, setLoopEnd,
        pixelsPerSample, setPixelsPerSample,
        automationMode, setAutomationMode,
        expandedTracks,
        markers, setMarkers,
        followPlayback, setFollowPlayback,
        timelineRef,
        samplesPerBeat,
        snapToGrid,
        fetchState,
        handleSplit,
        handleUndo,
        handleRedo,
        toggleExpand,
        toggleMute,
        toggleSolo,
        swing,
        handleSetSwing
    } = useTimelineLogic();

    const [editingClip, setEditingClip] = useState<any | null>(null);
    const [pianoRollData, setPianoRollData] = useState<{ trackIdx: number, clipId: string } | null>(null);
    const [draggingClip, setDraggingClip] = useState<any | null>(null);
    const [lasso, setLasso] = useState<{ startX: number, startY: number, endX: number, endY: number } | null>(null);
    const [scrollLeft, setScrollLeft] = useState(0);
    const [scrollTop, setScrollTop] = useState(0);
    const [containerWidth, setContainerWidth] = useState((window.innerWidth || 1024) - 300);
    const [containerHeight, setContainerHeight] = useState(600);
    const [trackHeights, setTrackHeights] = useState<Record<string, number>>({});
    const gridRef = useRef<HTMLDivElement>(null);
    const scrollRAF = useRef<number | null>(null);   // RAF handle for throttled scroll

    const getTrackDepth = (tList: any[], trackId: string) => {
        let depth = 0;
        let currentId = tList.find(t => t.id === trackId)?.parent_id;
        while (currentId) {
            depth++;
            currentId = tList.find(t => t.id === currentId)?.parent_id;
        }
        return depth;
    };

    const getTrackHeight = (trackId: string) => trackHeights[trackId] ?? 80;

    const handleHeightChange = (trackId: string, newHeight: number) => {
        setTrackHeights(prev => ({ ...prev, [trackId]: newHeight }));
    };

    const sensors = useSensors(
        useSensor(PointerSensor, {
            activationConstraint: {
                distance: 8, // Requires minimum 8px drag before taking over, allows clicking buttons
            },
        }),
        useSensor(KeyboardSensor, {
            coordinateGetter: sortableKeyboardCoordinates,
        })
    );

    const handleDragEnd = async (event: DragEndEvent) => {
        const { active, over } = event;

        if (over && active.id !== over.id) {
            const oldIndex = tracks.findIndex(t => t.id === active.id);
            const newIndex = tracks.findIndex(t => t.id === over.id);

            try {
                const targetTrack = tracks[newIndex];
                if (targetTrack.track_type === 'Folder') {
                    // Dropped on a folder, make it a child
                    await invoke('set_track_parent', { index: oldIndex, parentId: targetTrack.id });
                } else {
                    // Inherit parent of track dropped on
                    await invoke('set_track_parent', { index: oldIndex, parentId: targetTrack.parent_id });
                }

                await invoke('move_track', { from: oldIndex, to: newIndex });
                fetchState();
            } catch (e) {
                console.error('Track reorder failed:', e);
            }
        }
    };

    // RAF-throttled scroll handler: at most ONE React re-render per animation frame
    // regardless of how many scroll events the browser fires per frame (can be 5-10+).
    const onScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
        const el = e.currentTarget;
        if (scrollRAF.current !== null) return; // already scheduled — drop this event
        scrollRAF.current = requestAnimationFrame(() => {
            setScrollLeft(el.scrollLeft);
            setScrollTop(el.scrollTop);
            scrollRAF.current = null;
        });
    }, []);

    useEffect(() => {
        const updateWidth = () => {
            if (timelineRef.current) {
                const w = timelineRef.current.clientWidth;
                const h = timelineRef.current.clientHeight;
                setContainerWidth(w > 0 ? w - 150 : 874);
                setContainerHeight(h > 0 ? h : 600);
            }
        };
        updateWidth();
        window.addEventListener('resize', updateWidth);
        return () => {
            window.removeEventListener('resize', updateWidth);
            // Cancel any pending scroll RAF to avoid setState after unmount
            if (scrollRAF.current !== null) cancelAnimationFrame(scrollRAF.current);
        };
    }, [timelineRef]);

    const [clipContextMenu, setClipContextMenu] = useState<any | null>(null);
    const [timelineContextMenu, setTimelineContextMenu] = useState<any | null>(null);
    const [trackContextMenu, setTrackContextMenu] = useState<any | null>(null);

    // ── Global keyboard shortcuts (Only UI specific ones here, others in useTimelineLogic) ─────
    useEffect(() => {
        const onKey = async (e: KeyboardEvent) => {
            const tag = (document.activeElement?.tagName ?? '').toLowerCase();
            if (tag === 'input' || tag === 'textarea') return;

            // Escape — clear selection and close any menus
            if (e.key === 'Escape') {
                setSelectedClips(new Set());
                setClipContextMenu(null);
                setTimelineContextMenu(null);
                setTrackContextMenu(null);
                return;
            }

            // Ctrl+F — toggle follow playback (optional if not in logic, but it's in logic)
        };
        window.addEventListener('keydown', onKey);
        return () => window.removeEventListener('keydown', onKey);
    }, [setSelectedClips, setClipContextMenu, setTimelineContextMenu, setTrackContextMenu]);

    const onDragOver = (e: React.DragEvent) => {
        e.preventDefault();
        e.dataTransfer.dropEffect = "move";
        if (draggingClip && timelineRef.current) {
            const rect = timelineRef.current.getBoundingClientRect();
            const sl = timelineRef.current.scrollLeft;
            setDraggingClip({ ...draggingClip, x: e.clientX - rect.left + sl });
        }
    };

    const onDrop = async (e: React.DragEvent, trackIndexOverride?: number) => {
        setDraggingClip(null);
        e.preventDefault();
        e.stopPropagation();

        const files = Array.from(e.dataTransfer.files);
        if (files.length > 0) {
            const audioFiles = files.filter(f =>
                ['wav', 'mp3', 'flac', 'ogg', 'm4a'].some(ext => f.name.toLowerCase().endsWith(ext))
            );

            if (timelineRef.current && audioFiles.length > 0) {
                const rect = timelineRef.current.getBoundingClientRect();
                const sl = timelineRef.current.scrollLeft;
                const st = timelineRef.current.scrollTop;
                const dropX = e.clientX - rect.left + sl - 150;
                const startPos = Math.max(0, Math.floor(dropX / pixelsPerSample));

                let targetTrackIdx = trackIndexOverride;
                if (targetTrackIdx === undefined) {
                    const dropY = e.clientY - rect.top + st - 30;
                    // Find track using measurements
                    targetTrackIdx = trackMeasurements.measurements.findIndex(m => 
                        m.isVisible && dropY >= m.top && dropY < m.top + m.height
                    );
                }

                for (const file of audioFiles) {
                    try {
                        const filePath = (file as any).path || file.name;
                        const clipInfo = await invoke<any>('import_audio_file', { path: filePath });
                        if (targetTrackIdx !== undefined && targetTrackIdx >= 0 && targetTrackIdx < tracks.length) {
                            await invoke("add_clip_to_track", { trackIndex: targetTrackIdx, clipId: clipInfo.id, startPos });
                            targetTrackIdx++;
                        } else {
                            await invoke("create_track_with_clip", { clipId: clipInfo.id, startPos });
                        }
                    } catch (error) {
                        console.error("Load failed:", error);
                    }
                }
                fetchState();
            }
            return;
        }

        const clipId = e.dataTransfer.getData("vibe/clip-id");
        const pluginId = e.dataTransfer.getData("vibe/plugin-id");
        const moveClipId = e.dataTransfer.getData("vibe/move-clip-id");

        if (timelineRef.current) {
            const rect = timelineRef.current.getBoundingClientRect();
            const sl = timelineRef.current.scrollLeft;
            const st = timelineRef.current.scrollTop;

            if (clipId) {
                const dropX = e.clientX - rect.left + sl - 150;
                const dropY = e.clientY - rect.top + st - 30;
                const startPos = snapToGrid(Math.max(0, Math.floor(dropX / pixelsPerSample)));

                let targetTrackIdx = trackIndexOverride;
                if (targetTrackIdx === undefined) {
                    targetTrackIdx = trackMeasurements.measurements.findIndex(m => 
                        m.isVisible && dropY >= m.top && dropY < m.top + m.height
                    );
                }

                try {
                    if (targetTrackIdx >= 0 && targetTrackIdx < tracks.length) {
                        await invoke("add_clip_to_track", { trackIndex: targetTrackIdx, clipId, startPos });
                    } else {
                        await invoke("create_track_with_clip", { clipId: clipId, startPos });
                    }
                    fetchState();
                } catch (err) { console.error(err); }
            } else if (pluginId) {
                const dropY = e.clientY - rect.top + st - 30;
                let targetTrackIdx = trackIndexOverride;
                if (targetTrackIdx === undefined) {
                    targetTrackIdx = trackMeasurements.measurements.findIndex(m => 
                        m.isVisible && dropY >= m.top && dropY < m.top + m.height
                    );
                }

                try {
                    if (targetTrackIdx >= 0 && targetTrackIdx < tracks.length) {
                        await invoke("add_plugin_to_track", { trackIndex: targetTrackIdx, pluginPath: pluginId });
                    } else {
                        await invoke("create_track", { name: "Instrument", trackType: "MIDI" });
                        // Fetching latest tracks after adding track
                        const latestTracks = await invoke<any[]>("get_tracks");
                        await invoke("add_plugin_to_track", { trackIndex: latestTracks.length - 1, pluginPath: pluginId });
                    }
                    fetchState();
                } catch (err) { console.error(err); }
            } else if (moveClipId) {
                const targetIndex = trackIndexOverride ?? 0;
                const srcIdx = parseInt(e.dataTransfer.getData("vibe/move-src-idx"));
                const offset = parseFloat(e.dataTransfer.getData("vibe/move-offset"));
                const dropX = e.clientX - rect.left + sl - 150 - offset;
                const newPos = snapToGrid(Math.max(0, Math.floor(dropX / pixelsPerSample)));

                try {
                    // Logic for multi-clip movement
                    if (selectedClips.has(moveClipId)) {
                        const draggedClip = tracks[srcIdx].clips.find(c => c.id === moveClipId)
                            || tracks[srcIdx].midi_clips.find(c => c.id === moveClipId);

                        if (draggedClip) {
                            const sampleDelta = newPos - draggedClip.start_sample;
                            const trackDelta = targetIndex - srcIdx;

                            for (const id of selectedClips) {
                                // Find which track this selected clip belongs to
                                for (let i = 0; i < tracks.length; i++) {
                                    const t = tracks[i];
                                    const c = t.clips.find(clip => clip.id === id) || t.midi_clips.find(clip => clip.id === id);
                                    if (c) {
                                        const destT = Math.max(0, Math.min(tracks.length - 1, i + trackDelta));
                                        const destS = Math.max(0, c.start_sample + sampleDelta);
                                        await invoke("move_clip", {
                                            srcIdx: i,
                                            clipId: id,
                                            destIdx: destT,
                                            newPos: destS
                                        });
                                        break;
                                    }
                                }
                            }
                        }
                    } else {
                        // Single clip move
                        await invoke("move_clip", { srcIdx, clipId: moveClipId, destIdx: targetIndex, newPos });
                    }
                    fetchState();
                } catch (err) { console.error(err); }
            }
        }
    };

    const handleTimelineContextMenuAction = async (action: string, payload?: any) => {
        if (!timelineContextMenu) return;
        setTimelineContextMenu(null);
        switch (action) {
            case 'insert_track_audio': await invoke("create_track", { name: "New Audio", trackType: "Audio" }); break;
            case 'insert_track_midi': await invoke("create_track", { name: "New MIDI", trackType: "MIDI" }); break;
            case 'insert_track_folder': await invoke("create_track", { name: "New Folder", trackType: "Folder" }); break;
            case 'insert_silence': await invoke("insert_silence", { pos: Math.floor(playhead), len: Math.floor(samplesPerBeat * 4) }); break;
            case 'delete_time': await invoke("delete_time", { pos: loopStart, len: loopEnd - loopStart }); break;
            case 'duplicate_time': await invoke("duplicate_time", { pos: loopStart, len: loopEnd - loopStart }); break;
            case 'paste_time': await invoke("paste_time", { pos: Math.floor(playhead) }); break;
            case 'auto_loop':
                if (payload) {
                    let start = 0;
                    let end = 0;
                    if (payload.bars) {
                        const currentBar = Math.floor(playhead / (samplesPerBeat * 4));
                        start = currentBar * samplesPerBeat * 4;
                        end = start + payload.bars * samplesPerBeat * 4;
                    } else if (payload.beatFraction) {
                        const currentBeat = Math.floor(playhead / samplesPerBeat);
                        start = currentBeat * samplesPerBeat;
                        end = start + payload.beatFraction * samplesPerBeat;
                    }
                    start = Math.round(start);
                    end = Math.round(end);
                    setLoopStart(start);
                    setLoopEnd(end);
                    await invoke("set_loop_range", { start, end });
                }
                break;
            case 'snap_loop_zero_crossing':
                try {
                    const snapped: [number, number] = await invoke("snap_loop_to_zero", {
                        loopStart: Math.round(loopStart),
                        loopEnd: Math.round(loopEnd),
                        searchWindowMs: 20 // 20ms search window
                    });
                    setLoopStart(snapped[0]);
                    setLoopEnd(snapped[1]);
                    await invoke("set_loop_range", { start: snapped[0], end: snapped[1] });
                } catch (err) {
                    console.error("Zero crossing snap failed", err);
                    alert("Could not find suitable zero crossings in loop region.");
                }
                break;
            case 'tempo_detective':
                const durationSec = (loopEnd - loopStart) / 48000;
                if (durationSec <= 0.05) {
                    alert("Please select a wider loop range to analyze tempo.");
                    break;
                }
                // Estimate BPM assuming the loop represents 1 bar (4 beats)
                const estimatedBpm = Math.round((4 / durationSec) * 60 * 10) / 10;
                const apply = confirm(`📐 Tempo Detective:\n\nSelected loop region: ${durationSec.toFixed(3)} seconds.\nIf this region represents exactly 4 beats (1 bar):\nEstimated Tempo: ${estimatedBpm} BPM.\n\nApply this BPM to the project?`);
                if (apply) {
                    await invoke("set_bpm", { bpm: estimatedBpm });
                }
                break;
            case 'set_time_signature':
                const ts = prompt("New Time Signature:", "4/4");
                if (ts) {
                    const [n, d] = ts.split('/').map(Number);
                    if (n && d) await invoke("set_time_signature", { num: n, den: d });
                }
                break;
        }
        fetchState();
    };

    const handleTrackContextMenuAction = async (action: string, payload?: any) => {
        if (!trackContextMenu) return;
        setTrackContextMenu(null);
        const { trackIndex } = payload;
        try {
            switch (action) {
                case 'rename_track':
                    const name = prompt("New Track Name:", tracks[trackIndex]?.name);
                    if (name) await invoke("rename_track", { idx: trackIndex, name });
                    break;
                case 'duplicate_track': await invoke("duplicate_track", { idx: trackIndex }); break;
                case 'toggle_disable': await invoke("set_track_disabled", { index: trackIndex, disabled: !tracks[trackIndex].is_disabled }); break;
                case 'toggle_freeze': await invoke("set_track_frozen", { index: trackIndex, frozen: !tracks[trackIndex].is_frozen }); break;
                case 'arm_track': await invoke("set_track_arm", { index: trackIndex, armed: !tracks[trackIndex].is_armed }); break;
                case 'toggle_automation': toggleExpand(tracks[trackIndex].id); break;
                case 'add_child_track':
                    // Convert target track to a folder if it isn't already, then add a child
                    await invoke("set_track_type", { index: trackIndex, tType: "Folder" });
                    const childName = prompt("Child Track Name:", "New Track");
                    if (childName) {
                        await invoke("create_track_with_parent", { name: childName, trackType: "Audio", parentId: tracks[trackIndex].id });
                    }
                    break;
                case 'change_track_color':
                    const color = prompt("Hex Color:", tracks[trackIndex]?.color || "#FF0000");
                    if (color) await invoke("set_track_color", { idx: trackIndex, color });
                    break;
                case 'delete_track': if (confirm("Delete?")) await invoke("remove_track", { idx: trackIndex }); break;
            }
            fetchState();
        } catch (e) { console.error(e); }
    };

    const handleContextMenuAction = async (action: string, payload?: any) => {
        if (!clipContextMenu) return;
        const { clipId, trackIdx, isMidi } = clipContextMenu;
        setClipContextMenu(null);
        switch (action) {
            case 'rename':
                const currentName = [...tracks[trackIdx].clips, ...tracks[trackIdx].midi_clips].find(c => c.id === clipId)?.name;
                const newName = prompt("Rename:", currentName);
                if (newName) await invoke("rename_clip", { trackIdx, clipId, newName, isMidi });
                break;
            case 'duplicate': await invoke("duplicate_clip", { trackIdx, clipId, isMidi }); break;
            case 'consolidate': await invoke("consolidate_clip", { trackIdx, clipId, isMidi }); break;
            case 'delete':
                if (isMidi) await invoke("delete_midi_clip", { trackIdx, clipId });
                else await invoke("delete_clip", { trackIdx, clipId });
                break;
            case 'quick_gain':
                if (payload?.gain !== undefined) {
                    await invoke("set_clip_gain", { trackIdx, clipId, gain: payload.gain });
                }
                break;
            case 'set_display_mode':
                if (payload?.mode) {
                    localStorage.setItem(`vibe/clip-mode/${clipId}`, payload.mode);
                    // trigger quick local state redraw
                    fetchState();
                }
                break;
            case 'snap_to_grid': {
                const track = tracks[trackIdx];
                const clip = [...track.clips, ...track.midi_clips].find(c => c.id === clipId);
                if (clip) {
                    const start = clip.start_sample;
                    const len = ('duration_samples' in clip) ? clip.duration_samples : clip.length_samples;
                    const snappedStart = snapToGrid(start);
                    const snappedEnd = snapToGrid(start + len);
                    const snappedLen = Math.max(4800, snappedEnd - snappedStart);
                    
                    await invoke("move_clip", { 
                        srcIdx: trackIdx, 
                        clipId, 
                        destIdx: trackIdx, 
                        newPos: Math.round(snappedStart) 
                    });
                    
                    const offset = ('offset_in_data' in clip) ? clip.offset_in_data : 0;
                    await invoke("resize_clip", { 
                        trackIdx, 
                        clipId, 
                        newStart: Math.round(snappedStart), 
                        newOffset: offset, 
                        newLen: Math.round(snappedLen) 
                    });
                }
                break;
            }
            case 'statistics': {
                try {
                    const stats: any = await invoke("get_clip_statistics", { clipId });
                    alert(
                        `📊 Clip Statistics:\n\n` +
                        `Duration: ${(stats.duration_samples / 48000).toFixed(3)}s (${stats.duration_samples.toLocaleString()} samples)\n` +
                        `Peak Level: ${stats.peak_db.toFixed(2)} dBFS\n` +
                        `Average RMS: ${stats.rms_db.toFixed(2)} dBFS\n` +
                        `Crest Factor: ${stats.crest_factor.toFixed(2)} dB\n` +
                        `DC Offset: ${stats.dc_offset.toFixed(5)}`
                    );
                } catch (e) {
                    console.error(e);
                    alert("Could not load clip statistics (audio clips only).");
                }
                break;
            }
            case 'auto_crossfade': {
                const track = tracks[trackIdx];
                const clip = track.clips.find((c: any) => c.id === clipId);
                if (clip) {
                    const duration = clip.duration_samples || 0;
                    const end = clip.start_sample + duration;
                    const adjacent = track.clips.find((c: any) => c.id !== clipId && Math.abs(c.start_sample - end) < 4800);
                    if (adjacent) {
                        const crossfadeLen = 960; // 20ms
                        await invoke("set_crossfade", { trackIdx, clipId1: clip.id, clipId2: adjacent.id, fadeLen: crossfadeLen });
                    } else {
                        alert("No adjacent overlapping or abutting clips found on this track.");
                    }
                } else {
                    alert("Crossfade is only supported between audio clips.");
                }
                break;
            }
            case 'reverse': if (!isMidi) await invoke("reverse_audio_clip", { trackIdx, clipId }); break;
            case 'normalize': if (!isMidi) await invoke("normalize_clip", { trackIdx, clipId }); break;
            case 'set_color':
                if (payload?.color) await invoke("set_clip_color", { trackIdx, clipId, color: payload.color });
                break;
            case 'quantize':
                if (payload?.division) await invoke("quantize_notes", { trackIdx, clipId, division: payload.division });
                break;
            case 'humanize':
                if (payload) await invoke("humanize_midi_clip", { trackIdx, clipId, timing: payload.amount, velocity: payload.velocity });
                break;
            case 'apply_groove':
                if (payload?.template) await invoke("apply_groove_template", { trackIdx, clipId, templateName: payload.template });
                break;
            case 'set_warp_mode':
                if (payload?.mode) await invoke("set_audio_clip_warp_mode", { trackIdx, clipId, mode: payload.mode });
                break;
            case 'convert_audio_to_midi':
                await invoke("convert_audio_to_midi", { trackIdx, clipId, mode: "melodic" });
                break;
        }
        fetchState();
    };

    const trackMeasurements = useMemo(() => {
        const measurements: { id: string, index: number, top: number, height: number, isVisible: boolean }[] = [];
        let currentY = 0;

        for (let i = 0; i < tracks.length; i++) {
            const track = tracks[i];
            let isVisible = true;
            let currentParentId = track.parent_id;
            while (currentParentId) {
                const p = tracks.find((t: any) => t.id === currentParentId);
                if (!p) break;
                if (p.is_collapsed) {
                    isVisible = false;
                    break;
                }
                currentParentId = p.parent_id;
            }

            const laneHeight = getTrackHeight(track.id);
            // Height of automation group. TrackLane renders automation lanes if expanded.
            // Using approximate 100px.
            const autoHeight = expandedTracks.has(track.id) ? 100 : 0;
            const totalHeight = laneHeight + autoHeight;

            measurements.push({
                id: track.id,
                index: i,
                top: currentY,
                height: totalHeight,
                isVisible
            });

            if (isVisible) {
                currentY += totalHeight;
            }
        }

        return { measurements, totalHeight: currentY };
    }, [tracks, trackHeights, expandedTracks]);

    return (
        <div className="timeline-wrapper">
            <TimelineToolbar
                snap={snap} setSnap={setSnap}
                automationMode={automationMode} setAutomationMode={setAutomationMode}
                handleSplit={handleSplit} handleUndo={handleUndo} handleRedo={handleRedo}
                setPixelsPerSample={setPixelsPerSample}
                followPlayback={followPlayback} setFollowPlayback={setFollowPlayback}
                setSelectedClips={setSelectedClips}
                swing={swing} handleSetSwing={handleSetSwing}
            />

            <OverviewBar
                tracks={tracks} playhead={playhead}
                pixelsPerSample={pixelsPerSample} scrollLeft={scrollLeft} containerWidth={containerWidth}
                onScroll={(s) => { if (gridRef.current) gridRef.current.scrollLeft = s * pixelsPerSample; }}
            />

            <div
                className="timeline-container glass"
                ref={timelineRef}
                onScroll={(e) => setScrollLeft((e.target as HTMLElement).scrollLeft)}
                onWheel={(e) => {
                    if (e.ctrlKey || e.altKey) {
                        e.preventDefault();
                        const zoomFactor = -e.deltaY * 0.001;
                        if (timelineRef.current) {
                            const rect = timelineRef.current.getBoundingClientRect();
                            const sl = timelineRef.current.scrollLeft;
                            const scrollableX = e.clientX - rect.left - 150;
                            if (scrollableX >= 0) {
                                const contentX = scrollableX + sl;
                                const sampleUnderMouse = contentX / pixelsPerSample;
                                const newPixelsPerSample = Math.max(0.00001, Math.min(1.0, pixelsPerSample * (1 + zoomFactor)));
                                setPixelsPerSample(newPixelsPerSample);
                                const newContentX = sampleUnderMouse * newPixelsPerSample;
                                const newScrollLeft = newContentX - scrollableX;
                                requestAnimationFrame(() => {
                                    if (timelineRef.current) {
                                        timelineRef.current.scrollLeft = Math.max(0, newScrollLeft);
                                    }
                                });
                                return;
                            }
                        }
                        setPixelsPerSample(prev => Math.max(0.00001, Math.min(1.0, prev * (1 + zoomFactor))));
                    }
                }}
                onDragOver={onDragOver}
                onDrop={(e) => onDrop(e)}
                onMouseDown={(e) => {
                    const target = e.target as HTMLElement;
                    if (target.classList.contains('track-lane') || target.classList.contains('tracks-grid')) {
                        if (e.button !== 0) return;
                        const rect = timelineRef.current!.getBoundingClientRect();
                        const sX = e.clientX - rect.left + timelineRef.current!.scrollLeft;
                        const sY = e.clientY - rect.top + timelineRef.current!.scrollTop;
                        setLasso({ startX: sX, startY: sY, endX: sX, endY: sY });
                        if (!e.shiftKey) setSelectedClips(new Set());

                        const onMove = (mEv: MouseEvent) => {
                            const eX = mEv.clientX - rect.left + timelineRef.current!.scrollLeft;
                            const eY = mEv.clientY - rect.top + timelineRef.current!.scrollTop;
                            setLasso(prev => prev ? { ...prev, endX: eX, endY: eY } : null);
                        };
                        const onUp = () => {
                            window.removeEventListener('mousemove', onMove);
                            window.removeEventListener('mouseup', onUp);
                            setLasso(curr => {
                                if (!curr) return null;
                                const x1 = Math.min(curr.startX, curr.endX), x2 = Math.max(curr.startX, curr.endX);
                                const y1 = Math.min(curr.startY, curr.endY), y2 = Math.max(curr.startY, curr.endY);
                                const newSel = new Set(e.shiftKey ? selectedClips : []);
                                trackMeasurements.measurements.forEach((m) => {
                                    if (!m.isVisible) return;
                                    const tIdx = m.index;
                                    const t = tracks[tIdx];
                                    const top = 30 + m.top, btm = top + m.height;
                                    if (y2 >= top && y1 <= btm) {
                                        [t.clips, t.midi_clips].forEach((clipsList) => {
                                            clipsList.forEach(c => {
                                                const l = c.start_sample * pixelsPerSample + 150;
                                                const duration = ('duration_samples' in c) ? c.duration_samples : c.length_samples;
                                                const r = l + duration * pixelsPerSample;
                                                if (x2 >= l && x1 <= r) newSel.add(c.id);
                                            });
                                        });
                                    }
                                });
                                setSelectedClips(newSel);
                                return null;
                            });
                        };
                        window.addEventListener('mousemove', onMove);
                        window.addEventListener('mouseup', onUp);
                    }
                }}
            >
                <div className="timeline-ruler" onMouseDown={(e) => {
                    if (e.button !== 0) return; // Only left-click seeks playhead/drags loop
                    const rect = e.currentTarget.getBoundingClientRect();
                    const x = e.clientX - rect.left - 150 + scrollLeft;
                    const sPos = Math.max(0, x / pixelsPerSample);
                    invoke("set_playhead", { sample: Math.floor(sPos) });
                    const startS = sPos;
                    const onMove = (mEv: MouseEvent) => {
                        const cX = mEv.clientX - rect.left - 150 + scrollLeft;
                        const cS = Math.max(0, cX / pixelsPerSample);
                        setLoopStart(Math.min(startS, cS));
                        setLoopEnd(Math.max(startS, cS));
                    };
                    const onUp = (uEv: MouseEvent) => {
                        window.removeEventListener('mousemove', onMove);
                        window.removeEventListener('mouseup', onUp);
                        const endX = uEv.clientX - rect.left - 150 + scrollLeft;
                        const endS = Math.max(0, endX / pixelsPerSample);
                        if (Math.abs(endS - startS) > (samplesPerBeat / 8)) {
                            const s = Math.round(snapToGrid(Math.min(startS, endS)));
                            const e = Math.round(snapToGrid(Math.max(startS, endS)));
                            invoke("set_loop_range", { start: s, end: e });
                            setLoopStart(s); setLoopEnd(e);
                        }
                    };
                    window.addEventListener('mousemove', onMove);
                    window.addEventListener('mouseup', onUp);
                }}>
                    <div className="ruler-tracks-header">Tracks</div>
                    <div className="ruler-marks-wrapper">
                        <div className="ruler-marks-content" style={{ transform: `translateX(${-scrollLeft}px)` }}>
                            <TimelineRuler
                                pixelsPerSample={pixelsPerSample}
                                bpm={bpm}
                                width={10000}
                                onSeek={(s) => invoke("set_playhead", { sample: s })}
                                loopStart={loopStart}
                                loopEnd={loopEnd}
                                onSetLoopRange={async (s, e) => {
                                    setLoopStart(s);
                                    setLoopEnd(e);
                                    await invoke("set_loop_range", { start: Math.round(s), end: Math.round(e) });
                                }}
                                snapToGrid={snapToGrid}
                            />
                            {markers.map((m, idx) => (
                                <div key={m.id || idx} className="timeline-marker" style={{ left: `${m.pos * pixelsPerSample}px` }}>
                                    <span className="marker-label" style={{ backgroundColor: m.color || '#ff00dd' }}>🏷️ {m.label}</span>
                                </div>
                            ))}
                            <div className="loop-selection" style={{ left: `${loopStart * pixelsPerSample}px`, width: `${(loopEnd - loopStart) * pixelsPerSample}px` }} />
                            <div className="playhead-line" style={{ transform: `translateX(${playhead * pixelsPerSample}px)` }} />
                        </div>
                    </div>
                </div>

                <div className="tracks-grid" onScroll={onScroll} ref={gridRef} style={{ position: 'relative' }}>
                    <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
                        <SortableContext items={tracks.map(t => t.id)} strategy={verticalListSortingStrategy}>
                            <div style={{ position: 'relative', height: `${trackMeasurements.totalHeight}px`, width: '100%' }}>
                                {trackMeasurements.measurements.map(m => {
                                    if (!m.isVisible) return null;

                                    const overscan = 500; // pre-load half a screen above & below
                                    const isRendered = (m.top + m.height) >= scrollTop - overscan && m.top <= scrollTop + containerHeight + overscan;
                                    if (!isRendered) return null;

                                    const track = tracks[m.index];
                                    return (
                                        <SortableTrackItem key={track.id} track={track} top={m.top} depth={getTrackDepth(tracks, track.id)}>
                                            {({ dragRef, dragAttributes, dragListeners }) => (
                                                <TrackLane
                                                    track={track} index={m.index}
                                                    allTracks={tracks}
                                                    pixelsPerSample={pixelsPerSample} samplesPerBeat={samplesPerBeat}
                                                    selectedClips={selectedClips} setSelectedClips={setSelectedClips}
                                                    expandedTracks={expandedTracks} toggleExpand={toggleExpand}
                                                    toggleMute={toggleMute} toggleSolo={toggleSolo}
                                                    setTrackContextMenu={setTrackContextMenu} setClipContextMenu={setClipContextMenu}
                                                    setTimelineContextMenu={setTimelineContextMenu} setDraggingClip={setDraggingClip}
                                                    onDragOver={onDragOver} onDrop={onDrop}
                                                    snapToGrid={snapToGrid} fetchState={fetchState}
                                                    scrollLeft={scrollLeft} containerWidth={containerWidth}
                                                    playhead={playhead} automationMode={automationMode} bpm={bpm}
                                                    snap={snap}
                                                    setPixelsPerSample={setPixelsPerSample}
                                                    setEditingClip={setEditingClip} setPianoRollData={setPianoRollData}
                                                    trackHeight={getTrackHeight(track.id)}
                                                    onHeightChange={handleHeightChange}
                                                    dragRef={dragRef}
                                                    dragAttributes={dragAttributes}
                                                    dragListeners={dragListeners}
                                                />
                                            )}
                                        </SortableTrackItem>
                                    );
                                })}
                            </div>
                        </SortableContext>
                    </DndContext>
                </div>
            </div>

            {lasso && (
                <div className="lasso-box" style={{
                    left: Math.min(lasso.startX, lasso.endX),
                    top: Math.min(lasso.startY, lasso.endY),
                    width: Math.abs(lasso.startX - lasso.endX),
                    height: Math.abs(lasso.startY - lasso.endY)
                }} />
            )}

            {clipContextMenu && (
                <ClipContextMenu
                    x={clipContextMenu.x} y={clipContextMenu.y}
                    isMidi={clipContextMenu.isMidi}
                    clipId={clipContextMenu.clipId}
                    trackIndex={clipContextMenu.trackIdx}
                    onAction={handleContextMenuAction} onClose={() => setClipContextMenu(null)}
                />
            )}
            {timelineContextMenu && (
                <TimelineContextMenu
                    x={timelineContextMenu.x} y={timelineContextMenu.y}
                    onAction={handleTimelineContextMenuAction} onClose={() => setTimelineContextMenu(null)}
                />
            )}
            {trackContextMenu && (
                <TrackContextMenu
                    x={trackContextMenu.x} y={trackContextMenu.y}
                    trackIndex={trackContextMenu.trackIndex}
                    trackId={trackContextMenu.trackId}
                    onAction={(action) => handleTrackContextMenuAction(action, { trackIndex: trackContextMenu.trackIndex })}
                    onClose={() => setTrackContextMenu(null)}
                />
            )}

            {editingClip && <SampleEditor clip={editingClip} onClose={() => setEditingClip(null)} />}
            {pianoRollData && <PianoRoll trackIdx={pianoRollData.trackIdx} clipId={pianoRollData.clipId} onClose={() => setPianoRollData(null)} />}
        </div>
    );
};
