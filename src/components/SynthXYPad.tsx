import React, { useRef, useState, useEffect } from 'react';
import './SynthXYPad.css';

interface Parameter {
    id: string;
    name: string;
    value: number;
}

interface SynthXYPadProps {
    paramX?: Parameter;
    paramY?: Parameter;
    labelX?: string;
    labelY?: string;
    onUpdate: (id: string, value: number) => void;
}

export const SynthXYPad: React.FC<SynthXYPadProps> = ({
    paramX,
    paramY,
    labelX = "Timbre",
    labelY = "Space",
    onUpdate
}) => {
    const padRef = useRef<HTMLDivElement>(null);
    const [dragging, setDragging] = useState(false);

    // Use internal state for smooth dragging, sync with props when not dragging
    const [localX, setLocalX] = useState(0.5);
    const [localY, setLocalY] = useState(0.5);

    useEffect(() => {
        if (!dragging && paramX) setLocalX(paramX.value);
    }, [paramX, dragging]);

    useEffect(() => {
        if (!dragging && paramY) setLocalY(paramY.value);
    }, [paramY, dragging]);

    const handlePointerMove = (e: React.PointerEvent | PointerEvent) => {
        if (!dragging || !padRef.current) return;

        const rect = padRef.current.getBoundingClientRect();
        const xRaw = (e.clientX - rect.left) / rect.width;
        const yRaw = 1.0 - ((e.clientY - rect.top) / rect.height); // Y is usually 0 at bottom for parameters

        const x = Math.max(0, Math.min(1, xRaw));
        const y = Math.max(0, Math.min(1, yRaw));

        setLocalX(x);
        setLocalY(y);

        if (paramX) onUpdate(paramX.id, x);
        if (paramY) onUpdate(paramY.id, y);
    };

    const handlePointerDown = (e: React.PointerEvent) => {
        setDragging(true);
        // Compute initial click
        const rect = e.currentTarget.getBoundingClientRect();
        const xRaw = (e.clientX - rect.left) / rect.width;
        const yRaw = 1.0 - ((e.clientY - rect.top) / rect.height);

        const x = Math.max(0, Math.min(1, xRaw));
        const y = Math.max(0, Math.min(1, yRaw));

        setLocalX(x);
        setLocalY(y);

        if (paramX) onUpdate(paramX.id, x);
        if (paramY) onUpdate(paramY.id, y);

        e.currentTarget.setPointerCapture(e.pointerId);
    };

    const handlePointerUp = (e: React.PointerEvent) => {
        setDragging(false);
        e.currentTarget.releasePointerCapture(e.pointerId);
    };

    return (
        <div
            className="xy-pad-container"
            ref={padRef}
            onPointerDown={handlePointerDown}
            onPointerMove={handlePointerMove}
            onPointerUp={handlePointerUp}
        >
            <div className="xy-grid" />
            <div className="xy-axis-label xy-label-x">{labelX} &rarr;</div>
            <div className="xy-axis-label xy-label-y">&uarr; {labelY}</div>

            <div
                className="xy-puck"
                style={{
                    left: `${localX * 100}%`,
                    top: `${(1.0 - localY) * 100}%`
                }}
            />
        </div>
    );
};
