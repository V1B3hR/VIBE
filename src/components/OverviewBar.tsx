import React, { useRef, useEffect } from 'react';

interface Clip {
    id: string;
    name: string;
    start_sample: number;
    duration_samples: number;
}

interface Track {
    id: string;
    clips: Clip[];
    midi_clips: any[];
    color: string;
}

interface OverviewBarProps {
    tracks: Track[];
    playhead: number;
    pixelsPerSample: number;
    scrollLeft: number;
    containerWidth: number;
    onScroll: (sample: number) => void;
}

export const OverviewBar = ({
    tracks,
    playhead,
    pixelsPerSample,
    scrollLeft,
    containerWidth,
    onScroll,
}: OverviewBarProps) => {
    const canvasRef = useRef<HTMLCanvasElement>(null);

    // Dynamic project length: max clip end or 5 minutes
    const getProjectLength = () => {
        let max = 48000 * 60 * 5; // 5 mins default
        tracks.forEach(t => {
            t.clips.forEach(c => {
                max = Math.max(max, c.start_sample + c.duration_samples);
            });
            t.midi_clips?.forEach(c => {
                max = Math.max(max, c.start_sample + (c.length_samples || 0));
            });
        });
        return max + 48000 * 10; // +10s padding
    };

    const totalSamples = getProjectLength();

    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        // Handle high DPI
        const dpr = window.devicePixelRatio || 1;
        const rect = canvas.getBoundingClientRect();
        canvas.width = rect.width * dpr;
        canvas.height = rect.height * dpr;
        ctx.scale(dpr, dpr);

        ctx.clearRect(0, 0, rect.width, rect.height);

        const scaleX = rect.width / totalSamples;
        const trackHeight = rect.height / Math.max(1, tracks.length);

        // Draw Clips
        tracks.forEach((track, tIdx) => {
            ctx.fillStyle = track.color + '66';

            track.clips.forEach(clip => {
                const x = clip.start_sample * scaleX;
                const w = Math.max(1, clip.duration_samples * scaleX);
                ctx.fillRect(x, tIdx * trackHeight + 1, w, trackHeight - 2);
            });

            track.midi_clips?.forEach(clip => {
                const x = clip.start_sample * scaleX;
                const w = Math.max(1, (clip.length_samples || 0) * scaleX);
                ctx.fillRect(x, tIdx * trackHeight + 1, w, trackHeight - 2);
            });
        });

        // Draw Playhead
        ctx.strokeStyle = '#ff4d4d';
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(playhead * scaleX, 0);
        ctx.lineTo(playhead * scaleX, rect.height);
        ctx.stroke();

        // Draw Viewport Border (Visual hint of what's on screen)
        const viewportStartSample = scrollLeft / pixelsPerSample;
        const viewportSamples = containerWidth / pixelsPerSample;

        ctx.strokeStyle = 'rgba(255, 255, 255, 0.4)';
        ctx.lineWidth = 1.5;
        const vx = viewportStartSample * scaleX;
        const vw = viewportSamples * scaleX;
        ctx.strokeRect(vx, 0, vw, rect.height);

        ctx.fillStyle = 'rgba(255, 255, 255, 0.08)';
        ctx.fillRect(vx, 0, vw, rect.height);

    }, [tracks, playhead, totalSamples, pixelsPerSample, scrollLeft, containerWidth]);

    const handleClick = (e: React.MouseEvent) => {
        if (!canvasRef.current) return;
        const rect = canvasRef.current.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const sample = (x / rect.width) * totalSamples;
        onScroll(sample);
    };

    return (
        <div className="overview-bar-container" style={{
            height: '40px',
            background: 'rgba(0,0,0,0.6)',
            borderBottom: '1px solid rgba(255,255,255,0.1)',
            margin: '0 150px 0 150px', // Align with timeline area
            borderRadius: '4px 4px 0 0',
            overflow: 'hidden',
            cursor: 'pointer'
        }}>
            <canvas
                ref={canvasRef}
                style={{ width: '100%', height: '100%' }}
                onClick={handleClick}
            />
        </div>
    );
};
