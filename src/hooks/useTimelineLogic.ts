import { useState, useRef, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Track, Clip, Marker } from "../types/timeline";
import { listen } from "@tauri-apps/api/event";

export const useTimelineLogic = () => {
    const [tracks, setTracks] = useState<Track[]>([]);
    const [playhead, setPlayhead] = useState(0);
    const [bpm, setBpm] = useState(120);
    const [snap, setSnap] = useState(1); // 1 = 1/4 note, 4 = 1/16 note, 0 = off
    const [selectedClips, setSelectedClips] = useState<Set<string>>(new Set());
    const [loopStart, setLoopStart] = useState(0);
    const [loopEnd, setLoopEnd] = useState(48000 * 4);
    const [pixelsPerSample, setPixelsPerSample] = useState(0.0005);
    const [automationMode, setAutomationMode] = useState<'read' | 'draw' | 'erase'>('read');
    const [expandedTracks, setExpandedTracks] = useState<Set<string>>(new Set());
    const [markers, setMarkers] = useState<Marker[]>([]);
    const [followPlayback, setFollowPlayback] = useState(true);
    const [swing, setSwing] = useState(0.0);

    const timelineRef = useRef<HTMLDivElement>(null);
    const peaksCache = useRef<Map<string, number[][]>>(new Map());
    const playheadRef = useRef(playhead);
    const tracksRef = useRef(tracks);
    const selectedClipsRef = useRef(selectedClips);

    useEffect(() => { playheadRef.current = playhead; }, [playhead]);
    useEffect(() => { tracksRef.current = tracks; }, [tracks]);
    useEffect(() => { selectedClipsRef.current = selectedClips; }, [selectedClips]);

    const samplesPerBeat = (48000 * 60) / bpm;

    const snapToGrid = useCallback((samples: number) => {
        const snapSamples = snap === 0 ? samplesPerBeat : samplesPerBeat / snap;

        // Base grid snap
        const gridSnapped = snap === 0 ? samples : Math.round(samples / snapSamples) * snapSamples;

        // Snap to markers — magnetic pull if within ½ grid interval
        const threshold = snapSamples / 2;
        let markerSnapped = gridSnapped;
        let minDist = threshold;
        for (const marker of markers) {
            const dist = Math.abs(samples - marker.pos);
            if (dist < minDist) {
                minDist = dist;
                markerSnapped = marker.pos;
            }
        }
        return markerSnapped;
    }, [snap, samplesPerBeat, markers]);

    const fetchState = useCallback(async () => {
        try {
            const trackList = await invoke<Track[]>("get_tracks");
            const currentBpm = await invoke<number>("get_bpm");
            const currentMarkers = await invoke<any[]>("get_markers");

            trackList.forEach(track => {
                if (track.clips) {
                    track.clips.forEach(clip => {
                        if (!peaksCache.current.has(clip.id)) {
                            peaksCache.current.set(clip.id, clip.peaks);
                        }
                    });
                }
            });

            const [ls, le] = await invoke<[number, number]>("get_loop_range");

            setTracks(trackList);
            setBpm(currentBpm);
            setMarkers(currentMarkers);
            setLoopStart(ls);
            setLoopEnd(le);
        } catch (e) {
            console.error(e);
        }
    }, []);

    const updatePlayhead = useCallback(async () => {
        try {
            const ph = await invoke<number>("get_playhead");
            setPlayhead(ph);

            if (followPlayback && timelineRef.current) {
                const rect = timelineRef.current.getBoundingClientRect();
                const x = ph * pixelsPerSample + 150;
                const scrollLeft = timelineRef.current.scrollLeft;
                const width = rect.width;

                if (x > scrollLeft + width - 100 || x < scrollLeft) {
                    timelineRef.current.scrollLeft = x - width / 2;
                }
            }
        } catch (e) {
            console.error(e);
        }
    }, [followPlayback, pixelsPerSample]);

    useEffect(() => {
        fetchState();
        updatePlayhead();

        const setupListeners = async () => {
            const unlistenProject = await listen('project_updated', (event: any) => {
                const payload = event.payload;
                setTracks(payload.tracks);
                setBpm(payload.bpm);
                if (payload.swing !== undefined) setSwing(payload.swing);
                if (payload.markers) setMarkers(payload.markers);
            });

            const unlistenDrop = await listen('tauri://file-drop', async (event: any) => {
                const files = event.payload?.paths || event.payload;
                if (files && Array.isArray(files)) {
                    for (const filePath of files) {
                        try {
                            const clipInfo = await invoke<any>('import_audio_file', { path: filePath });
                            await invoke("create_track_with_clip", {
                                clipId: clipInfo.id,
                                startPos: Math.floor(playheadRef.current)
                            });
                        } catch (e) {
                            console.error("Drop Import Failed:", e);
                        }
                    }
                    fetchState();
                }
            });

            return () => {
                unlistenProject();
                unlistenDrop();
            };
        };

        const cleanupPromise = setupListeners();

        const playheadInterval = setInterval(updatePlayhead, 50);

        return () => {
            clearInterval(playheadInterval);
            cleanupPromise.then(cleanup => cleanup());
        };
    }, [fetchState, updatePlayhead]);

    const handleSplit = useCallback(async () => {
        const head = playheadRef.current;
        const currentTracks = tracksRef.current;
        const selected = selectedClipsRef.current;

        if (selected.size > 0) {
            for (const clipId of selected) {
                for (let i = 0; i < currentTracks.length; i++) {
                    const track = currentTracks[i];
                    const hasClip = track.clips.some(c => c.id === clipId) || track.midi_clips.some(c => c.id === clipId);
                    if (hasClip) {
                        try {
                            await invoke("slice_clip", {
                                trackIndex: i,
                                clipId: clipId,
                                samplePos: Math.floor(head)
                            });
                        } catch (err) {
                            console.error("Slice failed:", err);
                        }
                    }
                }
            }
        } else {
            for (let i = 0; i < currentTracks.length; i++) {
                const track = currentTracks[i];
                const clipToSlice = [...track.clips, ...track.midi_clips].find(clip => {
                    const len = ('duration_samples' in clip) ? (clip as any).duration_samples : (clip as any).length_samples;
                    return head >= clip.start_sample && head < clip.start_sample + len;
                });

                if (clipToSlice) {
                    try {
                        await invoke("slice_clip", {
                            trackIndex: i,
                            clipId: clipToSlice.id,
                            samplePos: Math.floor(head)
                        });
                    } catch (err) {
                        console.error("Slice failed:", err);
                    }
                }
            }
        }
    }, []);

    const handleUndo = useCallback(async () => {
        await invoke("undo");
    }, []);

    const handleRedo = useCallback(async () => {
        await invoke("redo");
    }, []);

    const toggleExpand = useCallback((id: string) => {
        setExpandedTracks(prev => {
            const next = new Set(prev);
            if (next.has(id)) next.delete(id);
            else next.add(id);
            return next;
        });
    }, []);

    const toggleMute = useCallback(async (index: number, currentMuted: boolean) => {
        await invoke("set_track_mute", { index, muted: !currentMuted });
        fetchState();
    }, [fetchState]);

    const toggleSolo = useCallback(async (index: number, currentSolo: boolean) => {
        await invoke("set_track_solo", { index, solo: !currentSolo });
        fetchState();
    }, [fetchState]);

    useEffect(() => {
        const handleKeyDown = async (e: KeyboardEvent) => {
            const target = e.target as HTMLElement;
            if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA') return;

            if (e.code === 'Space') {
                e.preventDefault();
                const isPlaying = await invoke<boolean>("is_playing");
                if (isPlaying) await invoke("pause_audio");
                else await invoke("play_audio");
            }

            if (e.key.toLowerCase() === 's') {
                handleSplit();
            }

            if (e.ctrlKey && e.key.toLowerCase() === 'd') {
                e.preventDefault();
                const selected = selectedClipsRef.current;
                const currentTracks = tracksRef.current;
                for (const clipId of selected) {
                    for (let trackIdx = 0; trackIdx < currentTracks.length; trackIdx++) {
                        const track = currentTracks[trackIdx];
                        const isAudio = track.clips.some(c => c.id === clipId);
                        const isMidi = track.midi_clips.some(c => c.id === clipId);
                        if (isAudio || isMidi) {
                            try {
                                await invoke("duplicate_clip", { trackIdx, clipId, isMidi });
                            } catch (e) {
                                console.error("Duplicate failed:", e);
                            }
                        }
                    }
                }
                fetchState();
            }

            if (e.ctrlKey && e.key.toLowerCase() === 'a') {
                e.preventDefault();
                const allIds = new Set<string>();
                tracksRef.current.forEach(t => {
                    t.clips.forEach(c => allIds.add(c.id));
                    t.midi_clips.forEach(c => allIds.add(c.id));
                });
                setSelectedClips(allIds);
            }

            if (e.ctrlKey && e.key.toLowerCase() === 'z') {
                e.preventDefault();
                if (e.shiftKey) await invoke("redo");
                else await invoke("undo");
            }
            if (e.ctrlKey && e.key.toLowerCase() === 'y') {
                e.preventDefault();
                await invoke("redo");
            }

            if (e.key === 'Delete' || e.key === 'Backspace') {
                const selected = selectedClipsRef.current;
                const currentTracks = tracksRef.current;
                for (const clipId of selected) {
                    for (let trackIdx = 0; trackIdx < currentTracks.length; trackIdx++) {
                        const track = currentTracks[trackIdx];
                        if (track.midi_clips.some(c => c.id === clipId)) {
                            await invoke("delete_midi_clip", { trackIdx, clipId });
                        } else if (track.clips.some(c => c.id === clipId)) {
                            await invoke("delete_clip", { trackIdx, clipId });
                        }
                    }
                }
                setSelectedClips(new Set());
                fetchState();
            }

            if (e.key === '=' || e.key === '+') {
                e.preventDefault();
                setPixelsPerSample(prev => {
                    const next = Math.min(1.0, prev * 1.5);
                    if (timelineRef.current) {
                        const rect = timelineRef.current.getBoundingClientRect();
                        const viewportWidth = rect.width - 150;
                        const newScrollLeft = (playheadRef.current * next) - viewportWidth / 2;
                        requestAnimationFrame(() => {
                            if (timelineRef.current) {
                                timelineRef.current.scrollLeft = Math.max(0, newScrollLeft);
                            }
                        });
                    }
                    return next;
                });
            }
            if (e.key === '-' || e.key === '_') {
                e.preventDefault();
                setPixelsPerSample(prev => {
                    const next = Math.max(0.00001, prev / 1.5);
                    if (timelineRef.current) {
                        const rect = timelineRef.current.getBoundingClientRect();
                        const viewportWidth = rect.width - 150;
                        const newScrollLeft = (playheadRef.current * next) - viewportWidth / 2;
                        requestAnimationFrame(() => {
                            if (timelineRef.current) {
                                timelineRef.current.scrollLeft = Math.max(0, newScrollLeft);
                            }
                        });
                    }
                    return next;
                });
            }

            if (e.key.toLowerCase() === 'l') {
                e.preventDefault();
                const current = await invoke<boolean>("is_loop_enabled");
                await invoke("set_loop_enabled", { enabled: !current });
                fetchState();
            }

            if (e.key.toLowerCase() === 'r') {
                e.preventDefault();
                await invoke("toggle_record");
            }

            if (e.code === 'Enter') {
                e.preventDefault();
                await invoke("set_playhead", { sample: 0 });
                updatePlayhead();
            }

            if (e.key.toLowerCase() === 'f') {
                e.preventDefault();
                setFollowPlayback(prev => !prev);
            }
        };

        window.addEventListener('keydown', handleKeyDown);
        return () => window.removeEventListener('keydown', handleKeyDown);
    }, [handleSplit, fetchState, updatePlayhead]);

    return {
        tracks, setTracks,
        playhead, setPlayhead,
        bpm, setBpm,
        snap, setSnap,
        selectedClips, setSelectedClips,
        loopStart, setLoopStart,
        loopEnd, setLoopEnd,
        pixelsPerSample, setPixelsPerSample,
        automationMode, setAutomationMode,
        expandedTracks, setExpandedTracks,
        markers, setMarkers,
        followPlayback, setFollowPlayback,
        swing,
        handleSetSwing: async (val: number) => {
            setSwing(val);
            await invoke("set_global_swing", { swing: val });
        },
        timelineRef,
        samplesPerBeat,
        snapToGrid,
        fetchState,
        updatePlayhead,
        handleSplit,
        handleUndo,
        handleRedo,
        toggleExpand,
        toggleMute,
        toggleSolo
    };
};
