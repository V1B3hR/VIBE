import { useRef, useEffect, useState, MouseEvent } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface AutomationKnot {
    sample_pos: number;
    value: number;
    tension?: number;
}

interface AutomationLaneProps {
    paramId: string;
    name: string;
    knots: AutomationKnot[];
    min: number;
    max: number;
    pixelsPerSample: number;
    height?: number;
    bpm: number;
    mode?: 'read' | 'draw' | 'erase';
}

export const AutomationLane = ({
    paramId, name, knots, min, max, pixelsPerSample, height = 60, bpm, mode = 'read'
}: AutomationLaneProps) => {
    const containerRef = useRef<HTMLDivElement>(null);
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const [isDrawing, setIsDrawing] = useState(false);
    const [gesturePoints, setGesturePoints] = useState<{ x: number, y: number }[]>([]);

    const [draggingKnot, setDraggingKnot] = useState<number | null>(null);
    const [draggingTension, setDraggingTension] = useState<{ idx: number, startY: number, startTension: number } | null>(null);

    // 1. Main Render Effect (Canvas)
    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        // Sync width to pixelsPerSample * large range or actual container
        canvas.width = pixelsPerSample * 48000 * 60 * 10 / bpm; // 10 minutes of audio
        canvas.height = height;

        ctx.clearRect(0, 0, canvas.width, canvas.height);

        // Draw Dynamic Grid (Matched to Timeline)
        const samplesPerBeat = (48000 * 60) / bpm;
        const pixelsPerBeat = samplesPerBeat * pixelsPerSample;
        const pixelsPerBar = pixelsPerBeat * 4;

        ctx.strokeStyle = 'rgba(255, 255, 255, 0.05)';
        ctx.lineWidth = 1;
        for (let x = 0; x < canvas.width; x += pixelsPerBeat) {
            ctx.beginPath();
            ctx.moveTo(x, 0);
            ctx.lineTo(x, height);
            ctx.stroke();
        }
        ctx.strokeStyle = 'rgba(255, 255, 255, 0.1)';
        for (let x = 0; x < canvas.width; x += pixelsPerBar) {
            ctx.beginPath();
            ctx.moveTo(x, 0);
            ctx.lineTo(x, height);
            ctx.stroke();
        }

        if (knots.length === 0) return;

        const sorted = [...knots].sort((a, b) => a.sample_pos - b.sample_pos);

        // Draw Curve
        ctx.beginPath();
        const startY = height - ((sorted[0].value - min) / (max - min) * height);
        ctx.moveTo(0, startY);
        ctx.lineTo(sorted[0].sample_pos * pixelsPerSample, startY);

        for (let i = 0; i < sorted.length - 1; i++) {
            const p0 = sorted[i];
            const p1 = sorted[i + 1];
            const x0 = p0.sample_pos * pixelsPerSample;
            const y0 = height - ((p0.value - min) / (max - min) * height);
            const x1 = p1.sample_pos * pixelsPerSample;
            const y1 = height - ((p1.value - min) / (max - min) * height);

            const tension = p0.tension || 0;
            if (Math.abs(tension) < 0.01) {
                ctx.lineTo(x1, y1);
            } else {
                const steps = 20; // smoothness of the drawn bezier
                for (let j = 1; j <= steps; j++) {
                    const t = j / steps;
                    const clamped = Math.max(-0.99, Math.min(0.99, tension));
                    const tCurved = clamped >= 0 ? Math.pow(t, 1 + clamped * 4) : Math.pow(t, 1 / (1 - clamped * 4));

                    const curX = x0 + t * (x1 - x0);
                    const curY = y0 + tCurved * (y1 - y0);
                    ctx.lineTo(curX, curY);
                }
            }
        }

        const last = sorted[sorted.length - 1];
        const lastY = height - ((last.value - min) / (max - min) * height);
        ctx.lineTo(canvas.width, lastY);

        ctx.lineWidth = 2;
        ctx.strokeStyle = '#a5b4fc';
        ctx.stroke();

        // Fill under curve
        ctx.lineTo(canvas.width, height);
        ctx.lineTo(0, height);
        ctx.closePath();
        const grad = ctx.createLinearGradient(0, 0, 0, height);
        grad.addColorStop(0, 'rgba(165, 180, 252, 0.1)');
        grad.addColorStop(1, 'rgba(165, 180, 252, 0)');
        ctx.fillStyle = grad;
        ctx.fill();

        // Draw Knots and Tension handles
        sorted.forEach((k, i) => {
            const x = k.sample_pos * pixelsPerSample;
            const y = height - ((k.value - min) / (max - min) * height);
            ctx.beginPath();
            ctx.arc(x, y, 4, 0, Math.PI * 2);
            ctx.fillStyle = (draggingKnot !== null && knots.indexOf(k) === draggingKnot) ? '#fff' : '#a5b4fc';
            ctx.fill();
            ctx.strokeStyle = '#fff';
            ctx.lineWidth = 1;
            ctx.stroke();

            // Draw Tension Handle for the segment
            if (i < sorted.length - 1) {
                const p1 = sorted[i + 1];
                const x1 = p1.sample_pos * pixelsPerSample;
                const y1 = height - ((p1.value - min) / (max - min) * height);

                const t = 0.5;
                const tension = k.tension || 0;
                const clamped = Math.max(-0.99, Math.min(0.99, tension));
                const tCurved = clamped >= 0 ? Math.pow(t, 1 + clamped * 4) : Math.pow(t, 1 / (1 - clamped * 4));

                const midX = x + t * (x1 - x);
                const midY = y + tCurved * (y1 - y);

                ctx.beginPath();
                ctx.arc(midX, midY, 3, 0, Math.PI * 2);
                ctx.fillStyle = (draggingTension?.idx === i) ? '#facc15' : 'rgba(165, 180, 252, 0.6)';
                ctx.fill();
                // invisible bigger hit area
                ctx.beginPath();
                ctx.arc(midX, midY, 8, 0, Math.PI * 2);
                // no fill
            }
        });

        // Draw Gesture
        if (isDrawing && gesturePoints.length > 1) {
            ctx.beginPath();
            ctx.moveTo(gesturePoints[0].x, gesturePoints[0].y);
            for (let i = 1; i < gesturePoints.length; i++) {
                ctx.lineTo(gesturePoints[i].x, gesturePoints[i].y);
            }
            ctx.strokeStyle = '#ffcc00';
            ctx.lineWidth = 3;
            ctx.setLineDash([5, 5]);
            ctx.stroke();
            ctx.setLineDash([]);
        }

    }, [knots, min, max, pixelsPerSample, height, isDrawing, gesturePoints, draggingKnot, bpm]);


    // --- Interaction Handling ---

    const handleMouseDown = (e: React.MouseEvent) => {
        if (!containerRef.current) return;
        const rect = containerRef.current.getBoundingClientRect();
        const mouseX = e.clientX - rect.left;
        const mouseY = e.clientY - rect.top;

        const sorted = [...knots].sort((a, b) => a.sample_pos - b.sample_pos);

        // 1. Check for tension handle hit first
        const tensionHitIdx = sorted.findIndex((k, i) => {
            if (i === sorted.length - 1) return false;
            const p1 = sorted[i + 1];
            const x0 = k.sample_pos * pixelsPerSample;
            const y0 = height - ((k.value - min) / (max - min) * height);
            const x1 = p1.sample_pos * pixelsPerSample;
            const y1 = height - ((p1.value - min) / (max - min) * height);

            const t = 0.5;
            const tension = k.tension || 0;
            const clamped = Math.max(-0.99, Math.min(0.99, tension));
            const tCurved = clamped >= 0 ? Math.pow(t, 1 + clamped * 4) : Math.pow(t, 1 / (1 - clamped * 4));

            const midX = x0 + t * (x1 - x0);
            const midY = y0 + tCurved * (y1 - y0);

            return Math.sqrt((midX - mouseX) ** 2 + (midY - mouseY) ** 2) < 8;
        });

        if (tensionHitIdx !== -1) {
            if (e.button === 2) {
                e.preventDefault();
                invoke("set_automation_tension", { paramId, timeSamples: sorted[tensionHitIdx].sample_pos, tension: 0.0 });
                return;
            }
            setDraggingTension({ idx: tensionHitIdx, startY: mouseY, startTension: sorted[tensionHitIdx].tension || 0 });
            return;
        }

        // 2. Check for knot hit
        const hitIdx = knots.findIndex(k => {
            const x = k.sample_pos * pixelsPerSample;
            const y = height - ((k.value - min) / (max - min) * height);
            return Math.sqrt((x - mouseX) ** 2 + (y - mouseY) ** 2) < 8;
        });

        if (hitIdx !== -1) {
            if (e.button === 2) { // Right Click to delete
                e.preventDefault();
                invoke("delete_automation_point", { paramId, pos: knots[hitIdx].sample_pos });
                return;
            }
            // Start Dragging
            setDraggingKnot(hitIdx);
            return;
        }

        if ((e.altKey || mode === 'draw') && e.button === 0) {
            // Start Gesture
            setIsDrawing(true);
            setGesturePoints([{ x: mouseX, y: mouseY }]);
        }
    };

    const handleMouseMove = (e: React.MouseEvent) => {
        if (!containerRef.current) return;
        const rect = containerRef.current.getBoundingClientRect();
        const mouseX = e.clientX - rect.left;
        const mouseY = e.clientY - rect.top;

        if (isDrawing) {
            setGesturePoints((prev: { x: number, y: number }[]) => [...prev, { x: mouseX, y: mouseY }]);
        } else if (draggingKnot !== null) {
            // Drag Knot
            const samplePos = Math.max(0, Math.floor(mouseX / pixelsPerSample));
            const normY = Math.max(0, Math.min(1, (height - mouseY) / height));
            const value = min + normY * (max - min);

            // Update in backend (debounced or real-time)
            // For now, let's try direct update (it might be heavy, but VIBE is fast)
            invoke("update_automation_point", {
                paramId,
                oldPos: knots[draggingKnot].sample_pos,
                newPos: samplePos,
                newValue: value
            });
        } else if (draggingTension !== null) {
            const sorted = [...knots].sort((a, b) => a.sample_pos - b.sample_pos);
            const deltaY = mouseY - draggingTension.startY;
            // Negative deltaY (mouse up) -> bow up usually.
            // But let's just make dragging down = positive curve, drag up = negative.
            const p0 = sorted[draggingTension.idx];
            const p1 = sorted[draggingTension.idx + 1];
            const y0 = height - ((p0.value - min) / (max - min) * height);
            const y1 = height - ((p1.value - min) / (max - min) * height);

            // Adjust depending on the slope so dragging "down" always bows it downwards
            const slopeDir = y1 > y0 ? -1 : 1;
            const newTension = Math.max(-0.99, Math.min(0.99, draggingTension.startTension + (deltaY / 50) * slopeDir));

            invoke("set_automation_tension", {
                paramId,
                timeSamples: p0.sample_pos,
                tension: newTension
            });
        }
    };

    const handleMouseUp = async () => {
        if (isDrawing) {
            setIsDrawing(false);
            if (gesturePoints.length > 5) await processGesture(gesturePoints);
            setGesturePoints([]);
        }
        setDraggingKnot(null);
        setDraggingTension(null);
    };

    const processGesture = async (points: { x: number, y: number }[]) => {
        if (!containerRef.current) return;
        const step = 5;
        for (let i = 0; i < points.length; i += step) {
            const p = points[i];
            const samplePos = Math.floor(p.x / pixelsPerSample);
            const normY = Math.max(0, Math.min(1, (height - p.y) / height));
            const val = min + normY * (max - min);
            try {
                await invoke("add_automation_point", { paramId, pos: samplePos, value: val });
            } catch (e) { }
        }
    };

    const handleDoubleClick = async (e: React.MouseEvent) => {
        if (!containerRef.current || e.altKey) return;
        const rect = containerRef.current.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;

        let samplePos = x / pixelsPerSample;
        const samplesPerBeat = (60.0 / bpm) * 48000;
        // Snap to grid on double click
        samplePos = Math.round(samplePos / (samplesPerBeat / 4)) * (samplesPerBeat / 4);

        const normalizedY = 1.0 - (Math.min(height, Math.max(0, y)) / height);
        const value = min + normalizedY * (max - min);

        try {
            await invoke("add_automation_point", { paramId, pos: Math.floor(samplePos), value });
        } catch (err) {
            console.error("Failed to add automation point:", err);
        }
    };

    return (
        <div
            className="automation-lane"
            ref={containerRef}
            onMouseDown={handleMouseDown}
            onMouseMove={handleMouseMove}
            onMouseUp={handleMouseUp}
            onMouseLeave={handleMouseUp}
            onDoubleClick={handleDoubleClick}
            onContextMenu={(e: React.MouseEvent) => e.preventDefault()}
            title="Double-click to add node | Alt+Drag to Draw | Right-click node to delete"
            style={{
                height: `${height}px`,
                position: 'relative',
                overflow: 'hidden',
                background: 'rgba(20, 20, 25, 0.4)',
                borderTop: '1px solid rgba(255,255,255,0.05)',
                cursor: isDrawing || mode === 'draw' ? 'crosshair' : 'default'
            }}
        >
            <div className="lane-header" style={{
                position: 'absolute',
                left: '4px',
                top: '2px',
                display: 'flex',
                gap: '8px',
                alignItems: 'center',
                zIndex: 5,
                pointerEvents: 'none',
                background: 'rgba(0,0,0,0.4)',
                padding: '2px 6px',
                borderRadius: '4px'
            }}>
                <span style={{ fontSize: '0.6rem', color: '#a5b4fc', fontWeight: 'bold', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
                    {name}
                </span>
                <span style={{ fontSize: '0.6rem', color: '#555' }}>
                    {min.toFixed(2)} - {max.toFixed(2)}
                </span>
            </div>

            <canvas
                ref={canvasRef}
                style={{ position: 'absolute', top: 0, left: 0, pointerEvents: 'none' }}
            />
        </div>
    );
};
