import * as React from 'react';
import { Track } from '../types/timeline';
import { ArrangementClip } from './ArrangementClip';
import { AutomationLane } from './AutomationLane';
import { TrackVUMeter } from './TrackVUMeter';
import { WaveformGL } from './WaveformGL';
import { invoke } from '@tauri-apps/api/core';

interface TrackLaneProps {
    track: Track;
    allTracks: Track[];
    index: number;
    pixelsPerSample: number;
    samplesPerBeat: number;
    selectedClips: Set<string>;
    setSelectedClips: React.Dispatch<React.SetStateAction<Set<string>>>;
    expandedTracks: Set<string>;
    toggleExpand: (id: string) => void;
    toggleMute: (index: number, muted: boolean) => void;
    toggleSolo: (index: number, solo: boolean) => void;
    setTrackContextMenu: React.Dispatch<React.SetStateAction<any>>;
    setClipContextMenu: React.Dispatch<React.SetStateAction<any>>;
    setTimelineContextMenu: React.Dispatch<React.SetStateAction<any>>;
    setDraggingClip: React.Dispatch<React.SetStateAction<any>>;
    onDragOver: (e: React.DragEvent) => void;
    onDrop: (e: React.DragEvent, trackIdx: number) => void;
    snapToGrid: (samples: number) => number;
    fetchState: () => void;
    scrollLeft: number;
    containerWidth: number;
    playhead: number;
    automationMode: 'read' | 'draw' | 'erase';
    bpm: number;
    snap: number;
    setPixelsPerSample: React.Dispatch<React.SetStateAction<number>>;
    setEditingClip?: React.Dispatch<React.SetStateAction<any>>;
    setPianoRollData?: React.Dispatch<React.SetStateAction<any>>;
    // Track resize
    trackHeight: number;
    onHeightChange: (id: string, newHeight: number) => void;

    // dnd-kit props
    dragRef?: (node: HTMLElement | null) => void;
    dragAttributes?: any;
    dragListeners?: any;
}

export const TrackLane: React.FC<TrackLaneProps> = ({
    track,
    index,
    allTracks,
    pixelsPerSample,
    samplesPerBeat,
    selectedClips,
    setSelectedClips,
    expandedTracks,
    toggleExpand,
    toggleMute,
    toggleSolo,
    setTrackContextMenu,
    setClipContextMenu,
    setTimelineContextMenu,
    setDraggingClip,
    onDragOver,
    onDrop,
    snapToGrid,
    fetchState,
    scrollLeft,
    containerWidth,
    playhead,
    automationMode,
    bpm,
    snap,
    setPixelsPerSample,
    setEditingClip,
    setPianoRollData,
    trackHeight,
    onHeightChange,
    dragRef,
    dragAttributes,
    dragListeners,
}) => {
    const renameTrack = async (idx: number) => {
        const name = prompt("New Track Name:", track.name);
        if (name) {
            await invoke("rename_track", { idx, name });
            fetchState();
        }
    };

    const [localVol, setLocalVol] = React.useState(track.volume.value);

    React.useEffect(() => {
        setLocalVol(track.volume.value);
    }, [track.volume.value]);

    const volToPct = (db: number) => {
        const min = track.volume.min_value || -60;
        const max = track.volume.max_value || 6;
        return Math.max(0, Math.min(100, ((db - min) / (max - min)) * 100));
    };

    const handleVolMouseDown = (e: React.MouseEvent) => {
        const rect = e.currentTarget.getBoundingClientRect();
        const update = (clientX: number) => {
            const pct = Math.max(0, Math.min(100, ((clientX - rect.left) / rect.width) * 100));
            const min = track.volume.min_value || -60;
            const max = track.volume.max_value || 6;
            const newVol = min + (pct / 100) * (max - min);
            setLocalVol(newVol);
            invoke("set_track_volume", { index, volume: newVol });
        };
        update(e.clientX);

        const onMove = (mEv: MouseEvent) => update(mEv.clientX);
        const onUp = () => {
            window.removeEventListener('mousemove', onMove);
            window.removeEventListener('mouseup', onUp);
            fetchState();
        };
        window.addEventListener('mousemove', onMove);
        window.addEventListener('mouseup', onUp);
    };

    const volColor = `hsl(${Math.max(0, 140 - volToPct(localVol) * 1.4)}, 100%, 50%)`;

    const snapPixels = (samplesPerBeat / (snap || 1)) * pixelsPerSample;
    const showSubGrid = snapPixels > 5 && snap > 1;

    const handleResizeMouseDown = (e: React.MouseEvent) => {
        e.preventDefault();
        e.stopPropagation();
        const startY = e.clientY;
        const startH = trackHeight;
        const onMove = (mEv: MouseEvent) => {
            const newH = Math.max(48, startH + mEv.clientY - startY);
            onHeightChange(track.id, newH);
        };
        const onUp = () => {
            window.removeEventListener('mousemove', onMove);
            window.removeEventListener('mouseup', onUp);
        };
        window.addEventListener('mousemove', onMove);
        window.addEventListener('mouseup', onUp);
    };

    return (
        <div
            className={`track-container-outer ${track.is_disabled ? 'track-disabled' : ''} ${track.is_frozen ? 'track-frozen' : ''}`}
            data-testid={`track-${index}`}
        >
            <div className="track-row" style={{ height: `${trackHeight}px` }}>
                <div
                    className="track-controls"
                    style={{ borderLeft: `4px solid ${track.color}`, touchAction: 'none' }}
                    ref={dragRef}
                    {...dragAttributes}
                    {...dragListeners}
                    onContextMenu={(e) => {
                        e.preventDefault();
                        setTrackContextMenu({
                            x: e.clientX,
                            y: e.clientY,
                            trackIndex: index,
                            trackId: track.id
                        });
                    }}
                >
                    <div className="track-header-top">
                        <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
                            {track.track_type === 'Folder' && (
                                <button
                                    onClick={(e) => { e.stopPropagation(); invoke("set_track_collapsed", { index, collapsed: !track.is_collapsed }).then(() => fetchState()); }}
                                    style={{
                                        background: 'transparent', border: 'none', color: '#fff', cursor: 'pointer',
                                        padding: '0 4px', fontSize: '10px', opacity: 0.8
                                    }}
                                    title={track.is_collapsed ? "Expand Folder" : "Collapse Folder"}
                                >
                                    {track.is_collapsed ? '►' : '▼'}
                                </button>
                            )}
                            <div className="track-name-row">
                                <span className="track-icon">
                                    {track.track_type === 'Folder' ? (track.is_collapsed ? '📁' : '📂') :
                                        track.track_type === 'MIDI' ? '🎹' : '🔊'}
                                </span>
                                <span className="track-name" onDoubleClick={() => renameTrack(index)}>{track.name}</span>
                                {track.track_type === 'Folder' && track.is_collapsed && (() => {
                                    const childCount = allTracks.filter(t => t.parent_id === track.id).length;
                                    return childCount > 0 ? (
                                        <span className="folder-child-count" title={`${childCount} track${childCount > 1 ? 's' : ''} inside`}>
                                            {childCount}
                                        </span>
                                    ) : null;
                                })()}
                            </div>
                        </div>
                        <div className="track-zoom-controls">
                            <button className="btn-zoom-tiny" onClick={() => setPixelsPerSample(p => p * 1.5)} title="Zoom In">+</button>
                            <button className="btn-zoom-tiny" onClick={() => setPixelsPerSample(p => p / 1.5)} title="Zoom Out">-</button>
                        </div>
                    </div>

                    <div className="track-volume-row" style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                        <div className="track-volume-slider-container" style={{ flex: 1 }} onMouseDown={handleVolMouseDown}>
                            <div
                                className="track-volume-bar-fill"
                                style={{
                                    width: `${volToPct(localVol)}%`,
                                    backgroundColor: volColor
                                }}
                            />
                            <span className="volume-value-tooltip">{localVol.toFixed(1)} dB</span>
                        </div>
                        <TrackVUMeter trackId={track.id} color={track.color} height={trackHeight - 36} />
                    </div>

                    <div className="track-buttons">
                        <button
                            className={`btn-control mute ${track.is_muted ? 'active' : ''}`}
                            onClick={() => toggleMute(index, track.is_muted)}
                            title="Mute Track"
                            data-testid={`track-mute-${index}`}
                        >
                            M
                        </button>
                        <button
                            className={`btn-control solo ${track.is_solo ? 'active' : ''}`}
                            onClick={() => toggleSolo(index, track.is_solo || false)}
                            title="Solo Track"
                            data-testid={`track-solo-${index}`}
                        >
                            S
                        </button>
                        <button
                            className={`btn-control automation ${expandedTracks.has(track.id) ? 'active' : ''}`}
                            onClick={() => toggleExpand(track.id)}
                            title="Show Automation"
                        >
                            A
                        </button>
                        <button
                            className="btn-control midi-add"
                            onClick={async () => {
                                const newClip = {
                                    id: crypto.randomUUID(),
                                    name: "New MIDI Pattern",
                                    start_sample: Math.floor(playhead),
                                    length_samples: 48000 * 4,
                                    notes: [],
                                    cc_events: [],
                                    color: track.color,
                                    is_muted: false,
                                    is_looped: false
                                };
                                await invoke("add_midi_clip", { trackIdx: index, clip: newClip });
                            }}
                            title="Add MIDI Clip"
                            data-testid={`track-add-midi-${index}`}
                        >
                            M+
                        </button>
                        {track.take_count !== undefined && track.take_count > 0 && (
                            <button
                                className={`btn-control comp ${track.comp_mode_enabled ? 'active' : ''}`}
                                onClick={() => {
                                    invoke("set_comp_mode", { trackIdx: index, enabled: !track.comp_mode_enabled }).then(() => fetchState());
                                }}
                                title="Show Comp Lanes"
                            >
                                ≡
                            </button>
                        )}
                    </div>
                    {/* Resize handle */}
                    <div
                        className="track-resize-handle"
                        onMouseDown={handleResizeMouseDown}
                        title="Przeciągnij aby zmienić wysokość ścieżki"
                    />
                </div>
                <div
                    className="track-lane"
                    onDragOver={onDragOver}
                    onDrop={(e) => onDrop(e, index)}
                    data-testid={`track-lane-${index}`}
                    style={{
                        height: `${trackHeight}px`,
                        backgroundSize: `${samplesPerBeat * 4 * pixelsPerSample}px 100%, ${samplesPerBeat * pixelsPerSample}px 100%, ${snapPixels}px 100%`,
                        backgroundImage: `
                            linear-gradient(90deg, rgba(255, 255, 255, 0.12) 1px, transparent 1px),
                            linear-gradient(90deg, rgba(255, 255, 255, 0.05) 1px, transparent 1px),
                            ${showSubGrid ? 'linear-gradient(90deg, rgba(255, 255, 255, 0.02) 1px, transparent 1px)' : 'none'}
                        `,
                        backgroundPosition: '0 0, 0 0, 0 0'
                    }}
                    onContextMenu={(e) => {
                        if (e.target === e.currentTarget) {
                            e.preventDefault();
                            setTimelineContextMenu({ x: e.clientX, y: e.clientY });
                        }
                    }}
                >
                    {/* Folder Aggregated Waveform / Ghost Clips */}
                    {track.track_type === 'Folder' && track.is_collapsed && (
                        <div className="folder-aggregated-waveform">
                            {allTracks
                                .filter(t => {
                                    // Recursively check if `t` is a descendant of this folder
                                    let p = t.parent_id;
                                    while (p) {
                                        if (p === track.id) return true;
                                        const parent = allTracks.find(parentTrack => parentTrack.id === p);
                                        p = parent?.parent_id;
                                    }
                                    return false;
                                })
                                .map(childTrack => (
                                    <React.Fragment key={`aggregate-${childTrack.id}`}>
                                        {[...(childTrack.clips || []), ...(childTrack.midi_clips || [])].filter(clip => {
                                            const duration = ('duration_samples' in clip) ? clip.duration_samples : clip.length_samples;
                                            const start = clip.start_sample;
                                            const end = start + duration;
                                            const viewStart = scrollLeft / pixelsPerSample;
                                            const viewEnd = (scrollLeft + containerWidth) / pixelsPerSample;
                                            return start <= viewEnd + 48000 && end >= Math.max(0, viewStart - 48000);
                                        }).map(clip => {
                                            const duration = ('duration_samples' in clip) ? clip.duration_samples : clip.length_samples;
                                            const clipWidthPx = duration * pixelsPerSample;
                                            const isMidiGhost = 'notes' in clip || 'preview_notes' in clip;
                                            const ghostColor = childTrack.color || '#888';
                                            const showLabel = clipWidthPx > 36;
                                            return (
                                                <div
                                                    key={`agg-clip-${clip.id}`}
                                                    className="ghost-clip-wrapper"
                                                    title={`${childTrack.name} — ${clip.name || (isMidiGhost ? 'MIDI' : 'Audio')}`}
                                                    style={{
                                                        left: `${clip.start_sample * pixelsPerSample}px`,
                                                        width: `${clipWidthPx}px`,
                                                        color: ghostColor,
                                                        background: `${ghostColor}22`,
                                                        borderColor: `${ghostColor}88`,
                                                    }}
                                                >
                                                    {showLabel && (
                                                        <div className="ghost-clip-label">
                                                            <span className="ghost-clip-label-track">{childTrack.name}</span>
                                                            {clip.name && clip.name !== childTrack.name && (
                                                                <span className="ghost-clip-label-clip"> · {clip.name}</span>
                                                            )}
                                                        </div>
                                                    )}
 
                                                    {isMidiGhost && (clip as any).preview_notes && (
                                                        <div className="midi-preview">
                                                            {(clip as any).preview_notes!.map((n: [number, number, number], ni: number) => (
                                                                <div
                                                                    key={ni}
                                                                    className="midi-dot"
                                                                    style={{
                                                                        left: `${(n[0] / (duration || 1)) * 100}%`,
                                                                        bottom: `${(n[1] / 127) * 100}%`,
                                                                        width: `${Math.max(2, (n[2] / (duration || 1)) * clipWidthPx)}px`,
                                                                        opacity: 0.85,
                                                                        background: ghostColor,
                                                                    }}
                                                                />
                                                            ))}
                                                        </div>
                                                    )}
 
                                                    {!isMidiGhost && (
                                                         <WaveformGL
                                                             clipId={clip.id}
                                                             width={clipWidthPx}
                                                             height={trackHeight}
                                                             color={ghostColor}
                                                             startSample={(clip as any).offset_in_data}
                                                             endSample={(clip as any).offset_in_data + duration}
                                                             totalSamples={0}
                                                             pixelsPerSample={pixelsPerSample}
                                                         />
                                                    )}
                                                </div>
                                            );
                                        })}
                                    </React.Fragment>
                                ))}
                        </div>
                    )}

                    {/* Original track clips (if folder not collapsed or regular track) */}
                    {(!track.is_collapsed || track.track_type !== 'Folder') && (
                        <>
                            {track.clips.filter(clip => {
                                const start = clip.start_sample;
                                const end = start + ((clip as any).duration_samples || (clip as any).length_samples || 0);
                                const viewStart = scrollLeft / pixelsPerSample;
                                const viewEnd = (scrollLeft + containerWidth) / pixelsPerSample;
                                return start <= viewEnd + 96000 && end >= Math.max(0, viewStart - 96000);
                            }).map(clip => (
                                <ArrangementClip
                                    key={clip.id}
                                    clip={clip}
                                    track={track}
                                    trackIdx={index}
                                    isMidi={false}
                                    pixelsPerSample={pixelsPerSample}
                                    selectedClips={selectedClips}
                                    setSelectedClips={setSelectedClips}
                                    setClipContextMenu={setClipContextMenu}
                                    setEditingClip={setEditingClip}
                                    setDraggingClip={setDraggingClip}
                                    snapToGrid={snapToGrid}
                                    fetchState={fetchState}
                                    scrollLeft={scrollLeft}
                                    containerWidth={containerWidth}
                                    trackHeight={trackHeight}
                                />
                            ))}

                            {track.midi_clips?.filter(clip => {
                                const start = clip.start_sample;
                                const end = start + ((clip as any).duration_samples || (clip as any).length_samples || 0);
                                const viewStart = scrollLeft / pixelsPerSample;
                                const viewEnd = (scrollLeft + containerWidth) / pixelsPerSample;
                                return start <= viewEnd + 96000 && end >= Math.max(0, viewStart - 96000);
                            }).map(clip => (
                                <ArrangementClip
                                    key={clip.id}
                                    clip={clip as any}
                                    track={track}
                                    trackIdx={index}
                                    isMidi={true}
                                    pixelsPerSample={pixelsPerSample}
                                    selectedClips={selectedClips}
                                    setSelectedClips={setSelectedClips}
                                    setClipContextMenu={setClipContextMenu}
                                    setPianoRollData={setPianoRollData}
                                    setDraggingClip={setDraggingClip}
                                    snapToGrid={snapToGrid}
                                    fetchState={fetchState}
                                    scrollLeft={scrollLeft}
                                    containerWidth={containerWidth}
                                    trackHeight={trackHeight}
                                />
                            ))}
                        </>
                    )}
                </div>
            </div>

            {expandedTracks.has(track.id) && (
                <div className="automation-group">
                    <div className="automation-controls-spacer" />
                    <div className="automation-lanes-container">
                        <AutomationLane
                            paramId={track.volume.id}
                            name="Main Volume"
                            knots={track.volume.automation}
                            min={track.volume.min_value}
                            max={track.volume.max_value}
                            pixelsPerSample={pixelsPerSample}
                            bpm={bpm}
                            mode={automationMode}
                        />
                    </div>
                </div>
            )}

            {track.comp_mode_enabled && track.comp_lanes && track.comp_lanes.map((lane, laneIdx) => (
                <div key={`comp-lane-${laneIdx}`} className="track-row comp-lane-row" style={{ height: `${trackHeight * 0.7}px`, opacity: 0.8, borderTop: '1px solid #333' }}>
                    <div className="track-controls" style={{ borderLeft: `4px solid ${track.color}`, backgroundColor: '#181818' }}>
                        <div className="track-header-top">
                            <span className="track-name" style={{ fontSize: '11px', color: '#aaa' }}>Take {laneIdx + 1}</span>
                            <button className="btn-control solo" onClick={() => invoke("set_active_take", { trackIdx: index, takeIdx: laneIdx }).then(() => fetchState())} title="Promote Take">
                                ↑
                            </button>
                        </div>
                    </div>
                    <div className="track-lane" style={{ height: `${trackHeight * 0.7}px`, backgroundColor: '#111' }}>
                        {lane.map(clip => (
                            <ArrangementClip
                                key={clip.id}
                                clip={clip}
                                track={track}
                                trackIdx={index}
                                isMidi={false} // Currently comping is Audio-focused
                                pixelsPerSample={pixelsPerSample}
                                selectedClips={selectedClips}
                                setSelectedClips={setSelectedClips}
                                setClipContextMenu={setClipContextMenu}
                                setEditingClip={setEditingClip}
                                setDraggingClip={setDraggingClip}
                                snapToGrid={snapToGrid}
                                fetchState={fetchState}
                                scrollLeft={scrollLeft}
                                containerWidth={containerWidth}
                                trackHeight={trackHeight * 0.7}
                            />
                        ))}
                    </div>
                </div>
            ))}
        </div>
    );
};
