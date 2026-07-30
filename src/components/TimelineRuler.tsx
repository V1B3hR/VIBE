import * as React from 'react';
import { useMemo } from 'react';

interface TimelineRulerProps {
    pixelsPerSample: number;
    bpm: number;
    sampleRate?: number;
    width: number;
    onSeek: (sample: number) => void;
    loopStart: number;
    loopEnd: number;
    onSetLoopRange: (start: number, end: number) => void;
    snapToGrid: (samples: number) => number;
}

export const TimelineRuler: React.FC<TimelineRulerProps> = ({
    pixelsPerSample,
    bpm,
    sampleRate = 48000,
    width,
    onSeek,
    loopStart,
    loopEnd,
    onSetLoopRange,
    snapToGrid
}) => {
    const samplesPerBeat = (sampleRate * 60) / bpm;
    const pixelsPerBeat = samplesPerBeat * pixelsPerSample;
    const pixelsPerBar = pixelsPerBeat * 4;

    const [dragging, setDragging] = React.useState<'start' | 'end' | null>(null);

    const handleDrag = (type: 'start' | 'end') => (e: React.MouseEvent) => {
        e.stopPropagation();
        setDragging(type);
        const onMove = (mEv: MouseEvent) => {
            const rect = (e.currentTarget.parentElement as HTMLElement).getBoundingClientRect();
            const x = mEv.clientX - rect.left;
            const samples = Math.max(0, x / pixelsPerSample);
            const snapped = snapToGrid(samples);

            if (type === 'start') {
                onSetLoopRange(Math.min(snapped, loopEnd - samplesPerBeat / 16), loopEnd);
            } else {
                onSetLoopRange(loopStart, Math.max(snapped, loopStart + samplesPerBeat / 16));
            }
        };
        const onUp = () => {
            setDragging(null);
            window.removeEventListener('mousemove', onMove);
            window.removeEventListener('mouseup', onUp);
        };
        window.addEventListener('mousemove', onMove);
        window.addEventListener('mouseup', onUp);
    };

    // Decide how many levels of subdivisions to show based on zoom
    const showBeats = pixelsPerBeat > 20;
    const showSubBeats8 = pixelsPerBeat > 80;
    const showSubBeats16 = pixelsPerBeat > 160;
    const showSubBeats32 = pixelsPerBeat > 320;
    const showSubBeats64 = pixelsPerBeat > 640;
    const showSubBeats128 = pixelsPerBeat > 1280;

    const showMillis = pixelsPerSample > 0.05;
    const showTensMillis = pixelsPerSample > 0.2;
    const showSingleMillis = pixelsPerSample > 1.0;
    const showSampleTicks = pixelsPerSample > 5.0;

    const markers = useMemo(() => {
        const result = [];
        const numBars = Math.ceil(width / pixelsPerBar) + 1;

        // 1. Musical grid: Bars, Beats, Sub-beats
        for (let bar = 0; bar < numBars; bar++) {
            const barX = bar * pixelsPerBar;
            result.push(
                <div key={`bar-${bar}`} className="ruler-mark bar-mark" style={{ left: `${barX}px` }}>
                    <span className="mark-label">{bar + 1}</span>
                </div>
            );

            if (showBeats) {
                for (let beat = 1; beat < 4; beat++) {
                    const beatX = barX + beat * pixelsPerBeat;
                    result.push(
                        <div key={`beat-${bar}-${beat}`} className="ruler-mark beat-mark" style={{ left: `${beatX}px` }}>
                            <span className="mark-label beat-label">{bar + 1}.{beat + 1}</span>
                        </div>
                    );

                    // 1/8 note subdivisions
                    if (showSubBeats8) {
                        const subX = beatX - pixelsPerBeat / 2;
                        result.push(<div key={`sub8-${bar}-${beat}`} className="ruler-mark sub-beat-mark" style={{ left: `${subX}px` }} />);
                    }
                }
                // Add the very first 1/8 sub-beat of the bar
                if (showSubBeats8) {
                    result.push(<div key={`sub8-${bar}-0`} className="ruler-mark sub-beat-mark" style={{ left: `${barX + pixelsPerBeat / 2}px` }} />);
                }
            }

            // More detailed musical grid (1/16, 1/32, 1/64, 1/128)
            if (showSubBeats16) {
                const step = pixelsPerBeat / 4;
                for (let i = 0; i < 16; i++) {
                    if (i % 4 !== 0) {
                        const x = barX + i * step;
                        result.push(<div key={`sub16-${bar}-${i}`} className="ruler-mark sub-beat-mark fine-mark" style={{ left: `${x}px` }} />);
                    }
                }
            }
            if (showSubBeats32) {
                const step = pixelsPerBeat / 8;
                for (let i = 0; i < 32; i++) {
                    if (i % 2 !== 0) {
                        const x = barX + i * step;
                        result.push(<div key={`sub32-${bar}-${i}`} className="ruler-mark sub-beat-mark fine-mark micro-mark" style={{ left: `${x}px` }} />);
                    }
                }
            }
            if (showSubBeats64) {
                const step = pixelsPerBeat / 16;
                for (let i = 0; i < 64; i++) {
                    if (i % 2 !== 0) {
                        const x = barX + i * step;
                        result.push(<div key={`sub64-${bar}-${i}`} className="ruler-mark sub-beat-mark fine-mark ultra-mark" style={{ left: `${x}px` }} />);
                    }
                }
            }
            if (showSubBeats128) {
                const step = pixelsPerBeat / 32;
                for (let i = 0; i < 128; i++) {
                    if (i % 2 !== 0) {
                        const x = barX + i * step;
                        result.push(<div key={`sub128-${bar}-${i}`} className="ruler-mark sub-beat-mark fine-mark nano-mark" style={{ left: `${x}px` }} />);
                    }
                }
            }
        }

        // 2. High-precision Time / Sample Grid
        if (showMillis) {
            // Absolute time indicators
            const pixelsPerSec = sampleRate * pixelsPerSample;
            const numSecs = Math.ceil(width / pixelsPerSec) + 1;
            
            // Draw every 100ms or 10ms or 1ms
            let msStep = 100; // default 100ms
            if (showSingleMillis) msStep = 1;
            else if (showTensMillis) msStep = 10;

            const samplesPerMs = sampleRate / 1000;
            const pixelsPerMs = samplesPerMs * pixelsPerSample;
            const stepPx = msStep * pixelsPerMs;
            const numSteps = Math.ceil(width / stepPx) + 1;

            for (let step = 0; step < numSteps; step++) {
                const ms = step * msStep;
                const secX = ms * pixelsPerMs;
                const totalSeconds = ms / 1000;
                
                // Formatting time like "mm:ss.ms" (e.g. 02:45.120) or just ".ms"
                const mins = Math.floor(totalSeconds / 60);
                const secs = Math.floor(totalSeconds % 60);
                const millis = ms % 1000;
                
                const timeLabel = mins > 0 
                    ? `${mins}:${secs.toString().padStart(2, '0')}.${millis.toString().padStart(3, '0')}`
                    : `${secs}.${millis.toString().padStart(3, '0')}s`;

                // Ticks for milliseconds (less prominent than bars/beats but clearly readable)
                const isSec = ms % 1000 === 0;
                const is100ms = ms % 100 === 0;

                result.push(
                    <div 
                        key={`ms-${ms}`} 
                        className={`ruler-mark ms-mark ${isSec ? 'sec-boundary' : is100ms ? 'ms-100' : 'ms-fine'}`} 
                        style={{ 
                            left: `${secX}px`,
                            borderLeft: isSec ? '1.5px solid rgba(0, 255, 204, 0.4)' : is100ms ? '1px dashed rgba(0, 255, 204, 0.2)' : '1px dotted rgba(255,255,255,0.08)'
                        }}
                    >
                        {(isSec || (showTensMillis && is100ms) || showSingleMillis) && (
                            <span className="mark-label ms-label" style={{ top: isSec ? '15px' : '20px', color: isSec ? '#0fc' : '#a8e063' }}>
                                {timeLabel}
                            </span>
                        )}
                    </div>
                );
            }
        }

        // 3. Sample index markers (zoomed in to raw wave samples level)
        if (showSampleTicks) {
            let sampleStep = 100; // default every 100 samples
            if (pixelsPerSample > 15.0) sampleStep = 10;
            if (pixelsPerSample > 50.0) sampleStep = 1;

            const stepPx = sampleStep * pixelsPerSample;
            const numSteps = Math.ceil(width / stepPx) + 1;

            for (let step = 0; step < numSteps; step++) {
                const sampleIdx = step * sampleStep;
                const sampleX = sampleIdx * pixelsPerSample;

                result.push(
                    <div 
                        key={`smpl-${sampleIdx}`} 
                        className="ruler-mark sample-mark" 
                        style={{ 
                            left: `${sampleX}px`, 
                            borderLeft: '1px solid rgba(255, 140, 66, 0.2)' 
                        }}
                    >
                        {(sampleIdx % (sampleStep * 5) === 0) && (
                            <span className="mark-label sample-label" style={{ top: '26px', color: '#ff8c42', fontSize: '8px' }}>
                                {sampleIdx.toLocaleString()} smp
                            </span>
                        )}
                    </div>
                );
            }
        }

        return result;
    }, [
        width,
        pixelsPerBar,
        pixelsPerBeat,
        showBeats,
        showSubBeats8,
        showSubBeats16,
        showSubBeats32,
        showSubBeats64,
        showSubBeats128,
        showMillis,
        showTensMillis,
        showSingleMillis,
        showSampleTicks,
        pixelsPerSample,
        sampleRate
    ]);


    return (
        <div className="ruler-content" style={{ width: `${width}px`, position: 'relative', height: '100%' }}>
            {markers}

            {/* Loop Region */}
            <div
                className="loop-region"
                style={{
                    position: 'absolute',
                    left: `${loopStart * pixelsPerSample}px`,
                    width: `${(loopEnd - loopStart) * pixelsPerSample}px`,
                    height: '100%',
                    backgroundColor: 'rgba(255, 215, 0, 0.15)',
                    borderLeft: '2px solid rgba(255, 215, 0, 0.8)',
                    borderRight: '2px solid rgba(255, 215, 0, 0.8)',
                    zIndex: 5,
                    pointerEvents: 'none'
                }}
            />

            {/* Loop Handles */}
            <div
                className="loop-handle start"
                onMouseDown={handleDrag('start')}
                style={{
                    position: 'absolute',
                    left: `${loopStart * pixelsPerSample - 4}px`,
                    width: '8px',
                    height: '100%',
                    cursor: 'ew-resize',
                    zIndex: 10,
                    display: 'flex',
                    justifyContent: 'center'
                }}
            >
                <div style={{ width: '2px', height: '60%', backgroundColor: 'var(--accent)', borderRadius: '1px', marginTop: '2px' }} />
            </div>

            <div
                className="loop-handle end"
                onMouseDown={handleDrag('end')}
                style={{
                    position: 'absolute',
                    left: `${loopEnd * pixelsPerSample - 4}px`,
                    width: '8px',
                    height: '100%',
                    cursor: 'ew-resize',
                    zIndex: 10,
                    display: 'flex',
                    justifyContent: 'center'
                }}
            >
                <div style={{ width: '2px', height: '60%', backgroundColor: 'var(--accent)', borderRadius: '1px', marginTop: '2px' }} />
            </div>
        </div>
    );
};
