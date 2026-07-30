import * as React from 'react';
import { Clip, MidiClip, Track } from '../types/timeline';
import { WaveformGL } from './WaveformGL';
import { invoke } from '@tauri-apps/api/core';

interface ArrangementClipProps {
    clip: Clip | MidiClip;
    track: Track;
    trackIdx: number;
    isMidi: boolean;
    pixelsPerSample: number;
    selectedClips: Set<string>;
    setSelectedClips: React.Dispatch<React.SetStateAction<Set<string>>>;
    setClipContextMenu: React.Dispatch<React.SetStateAction<any>>;
    setEditingClip?: React.Dispatch<React.SetStateAction<any>>;
    setPianoRollData?: React.Dispatch<React.SetStateAction<any>>;
    setDraggingClip: React.Dispatch<React.SetStateAction<any>>;
    snapToGrid: (samples: number) => number;
    fetchState: () => void;
    scrollLeft: number;
    containerWidth: number;
    trackHeight?: number;
}

export const ArrangementClip: React.FC<ArrangementClipProps> = ({
    clip,
    track,
    trackIdx,
    isMidi,
    pixelsPerSample,
    selectedClips,
    setSelectedClips,
    setClipContextMenu,
    setEditingClip,
    setPianoRollData,
    setDraggingClip,
    snapToGrid,
    fetchState,
    scrollLeft,
    containerWidth,
    trackHeight = 80,
}) => {
    const clipStartPx = clip.start_sample * pixelsPerSample;
    const duration = isMidi ? (clip as MidiClip).length_samples : (clip as Clip).duration_samples;
    const clipEndPx = (clip.start_sample + duration) * pixelsPerSample;
    const isVisible = clipEndPx >= scrollLeft && clipStartPx <= scrollLeft + containerWidth;

    if (!isVisible) return null;

    const handleSelect = (e: React.MouseEvent) => {
        e.stopPropagation();
        if (e.shiftKey) {
            const next = new Set(selectedClips);
            if (next.has(clip.id)) next.delete(clip.id);
            else next.add(clip.id);
            setSelectedClips(next);
        } else {
            setSelectedClips(new Set([clip.id]));
        }
    };

    const handleContextMenu = (e: React.MouseEvent) => {
        e.preventDefault();
        e.stopPropagation();
        setClipContextMenu({ x: e.clientX, y: e.clientY, clipId: clip.id, trackIdx, isMidi });
    };

    const handleDragStart = (e: React.DragEvent) => {
        if (e.altKey) {
            e.preventDefault();
            return;
        }
        e.dataTransfer.setData("vibe/move-clip-id", clip.id);
        e.dataTransfer.setData("vibe/move-src-idx", trackIdx.toString());

        const rect = e.currentTarget.getBoundingClientRect();
        const offset = e.clientX - rect.left;
        e.dataTransfer.setData("vibe/move-offset", offset.toString());

        setDraggingClip({
            id: clip.id,
            name: clip.name,
            color: track.color,
            duration: duration,
            offset,
            x: e.clientX,
            trackIdx: trackIdx
        });

        const img = new Image();
        img.src = 'data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7';
        e.dataTransfer.setDragImage(img, 0, 0);
    };

    const handleTrimMouseDown = (e: React.MouseEvent, side: 'left' | 'right') => {
        e.stopPropagation();
        const startX = e.clientX;
        const initialStart = clip.start_sample;
        const initialOffset = (clip as Clip).offset_in_data || 0;
        const initialLen = duration;

        const onMove = (moveEv: MouseEvent) => {
            const deltaSamples = (moveEv.clientX - startX) / pixelsPerSample;
            let newStart = initialStart;
            let newOffset = initialOffset;
            let newLen = initialLen;

            if (side === 'left') {
                newStart = snapToGrid(Math.max(0, initialStart + deltaSamples));
                newOffset = initialOffset + (newStart - initialStart);
                newLen = initialLen - (newStart - initialStart);
            } else {
                newLen = snapToGrid(Math.max(100, initialLen + deltaSamples));
            }

            invoke("resize_clip", {
                trackIdx,
                clipId: clip.id,
                newStart,
                newOffset,
                newLen
            }).then(fetchState);
        };

        const onUp = () => {
            window.removeEventListener('mousemove', onMove);
            window.removeEventListener('mouseup', onUp);
        };
        window.addEventListener('mousemove', onMove);
        window.addEventListener('mouseup', onUp);
    };

    const handleFadeMouseDown = (e: React.MouseEvent, side: 'in' | 'out') => {
        e.stopPropagation();
        const startX = e.clientX;
        const startLen = side === 'in' ? (clip as Clip).fade_in_len : (clip as Clip).fade_out_len;

        const onMove = (moveEvent: MouseEvent) => {
            const deltaSamples = (side === 'in' ? (moveEvent.clientX - startX) : (startX - moveEvent.clientX)) / pixelsPerSample;
            const newVal = Math.max(0, Math.floor(startLen + deltaSamples));

            invoke("set_clip_fades", {
                trackIdx,
                clipId: clip.id,
                inLen: side === 'in' ? newVal : (clip as Clip).fade_in_len,
                outLen: side === 'out' ? newVal : (clip as Clip).fade_out_len
            }).then(fetchState);
        };

        const onUp = () => {
            window.removeEventListener('mousemove', onMove);
            window.removeEventListener('mouseup', onUp);
        };

        window.addEventListener('mousemove', onMove);
        window.addEventListener('mouseup', onUp);
    };

    const handleSlipMouseDown = (e: React.MouseEvent) => {
        if (!e.altKey || isMidi) return;
        e.stopPropagation();
        e.preventDefault();

        const startX = e.clientX;
        const initialOffset = (clip as Clip).offset_in_data;

        const onMove = (moveEv: MouseEvent) => {
            const deltaSamples = (startX - moveEv.clientX) / pixelsPerSample;
            const newOffset = Math.max(0, Math.floor(initialOffset + deltaSamples));

            invoke("resize_clip", {
                trackIdx,
                clipId: clip.id,
                newStart: clip.start_sample,
                newOffset,
                newLen: duration
            }).then(fetchState);
        };

        const onUp = () => {
            window.removeEventListener('mousemove', onMove);
            window.removeEventListener('mouseup', onUp);
        };
        window.addEventListener('mousemove', onMove);
        window.addEventListener('mouseup', onUp);
    };

    const handleGainMouseDown = (e: React.MouseEvent) => {
        e.stopPropagation();
        e.preventDefault();
        const startY = e.clientY;
        const startGain = (clip as Clip).gain ?? 1.0;

        const onMove = (mEv: MouseEvent) => {
            const deltaY = startY - mEv.clientY; // up is positive gain
            const gainChange = deltaY / 100; // 100px = unity gain range
            const newGain = Math.max(0, Math.min(2.0, startGain + gainChange));

            invoke("set_clip_gain", { trackIdx, clipId: clip.id, gain: newGain })
                .then(fetchState);
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
            className={`${isMidi ? 'midi-clip' : 'audio-clip'} ${selectedClips.has(clip.id) ? 'selected' : ''}`}
            data-testid={`clip-${clip.id}`}
            onContextMenu={handleContextMenu}
            onClick={handleSelect}
            onMouseDown={handleSlipMouseDown}
            draggable
            onDragStart={handleDragStart}
            onDragEnd={() => setDraggingClip(null)}
            onDoubleClick={() => isMidi ? setPianoRollData?.({ trackIdx, clipId: clip.id }) : setEditingClip?.({ ...clip, trackIndex: trackIdx } as any)}
            style={{
                left: `${clipStartPx}px`,
                width: `${duration * pixelsPerSample}px`,
                background: `linear-gradient(180deg, ${clip.color || track.color}aa 0%, ${clip.color || track.color} 100%)`,
                ...(isMidi ? { borderTop: '2px solid rgba(255,255,255,0.3)' } : {})
            }}
        >
            <div className="clip-label">{clip.name}</div>

            {isMidi ? (
                <>
                    <div className="midi-preview">
                        {(clip as MidiClip).preview_notes?.map((n: [number, number, number], ni: number) => (
                            <div
                                key={ni}
                                className="midi-dot"
                                style={{
                                    left: `${(n[0] / duration) * 100}%`,
                                    bottom: `${(n[1] / 127) * 100}%`,
                                    width: `${Math.max(2, (n[2] / duration) * (duration * pixelsPerSample))}px`,
                                    opacity: 0.85,
                                    background: '#ffffff',
                                }}
                            />
                        ))}
                    </div>
                </>
            ) : (
                <>
                    {/* Fade In overlay visualizer */}
                    {(clip as Clip).fade_in_len > 0 && (
                        <div
                            className="fade-overlay fade-in-overlay"
                            style={{ width: `${(clip as Clip).fade_in_len * pixelsPerSample}px` }}
                        >
                            <svg className="fade-svg" preserveAspectRatio="none" viewBox="0 0 100 100">
                                <path d="M 0 100 Q 20 10 100 0 L 100 100 Z" fill="rgba(0, 0, 0, 0.55)" />
                                <path d="M 0 100 Q 20 10 100 0" stroke="rgba(255, 255, 255, 0.5)" strokeWidth="1.5" fill="none" vectorEffect="non-scaling-stroke" />
                            </svg>
                        </div>
                    )}

                    {/* Fade Out overlay visualizer */}
                    {(clip as Clip).fade_out_len > 0 && (
                        <div
                            className="fade-overlay fade-out-overlay"
                            style={{
                                width: `${(clip as Clip).fade_out_len * pixelsPerSample}px`,
                                right: 0
                            }}
                        >
                            <svg className="fade-svg" preserveAspectRatio="none" viewBox="0 0 100 100">
                                <path d="M 0 0 Q 80 10 100 100 L 100 0 Z" fill="rgba(0, 0, 0, 0.55)" />
                                <path d="M 0 0 Q 80 10 100 100" stroke="rgba(255, 255, 255, 0.5)" strokeWidth="1.5" fill="none" vectorEffect="non-scaling-stroke" />
                            </svg>
                        </div>
                    )}

                    <div style={{ width: '100%', height: '100%', position: 'absolute', top: 0, left: 0, zIndex: 1 }}>
                        <WaveformGL
                            clipId={clip.id}
                            width={duration * pixelsPerSample}
                            height={trackHeight}
                            color={clip.color || track.color}
                            startSample={(clip as Clip).offset_in_data}
                            endSample={(clip as Clip).offset_in_data + duration}
                            totalSamples={0}
                            pixelsPerSample={pixelsPerSample}
                        />
                    </div>

                    <div className="trim-handle trim-handle-left" onMouseDown={(e) => handleTrimMouseDown(e, 'left')} />
                    <div className="trim-handle trim-handle-right" onMouseDown={(e) => handleTrimMouseDown(e, 'right')} />
                </>
            )}

            {/* Clip Gain Handle */}
            {!isMidi && isVisible && (
                <div
                    className="clip-gain-handle"
                    onMouseDown={handleGainMouseDown}
                    style={{
                        bottom: `${Math.min(95, Math.max(5, ((clip as Clip).gain ?? 1.0) * 50))}%`
                    }}
                >
                    <div className="clip-gain-label">
                        {(20 * Math.log10((clip as Clip).gain || 0.0001)).toFixed(1)} dB
                    </div>
                </div>
            )}
        </div>
    );
};
