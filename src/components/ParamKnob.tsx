import React, { useState, useRef } from 'react';

interface Parameter {
    id: string;
    name: string;
    value: number;
    min_value: number;
    max_value: number;
}

interface ParamKnobProps {
    param: Parameter;
    onChange: (id: string, val: number) => void;
    size?: number;
    color?: string;
}

export const ParamKnob: React.FC<ParamKnobProps> = ({ param, onChange, size = 40, color = "#00e5ff" }) => {
    const startY = useRef(0);
    const startVal = useRef(0);

    // Normalize value to 0..1 for display
    const range = param.max_value - param.min_value;
    const normalized = (param.value - param.min_value) / (range || 1);

    const handleMouseDown = (e: React.MouseEvent) => {
        startY.current = e.clientY;
        startVal.current = param.value;
        document.body.style.cursor = 'ns-resize';
        window.addEventListener('mousemove', handleMouseMove);
        window.addEventListener('mouseup', handleMouseUp);
    };

    const handleMouseMove = (e: MouseEvent) => {
        const dy = startY.current - e.clientY;
        // Sensitivity: full range in 200px
        const deltaNormalized = dy / 200.0;
        const delta = deltaNormalized * range;

        let newVal = startVal.current + delta;
        newVal = Math.max(param.min_value, Math.min(param.max_value, newVal));

        onChange(param.id, newVal);
    };

    const handleMouseUp = () => {
        document.body.style.cursor = 'default';
        window.removeEventListener('mousemove', handleMouseMove);
        window.removeEventListener('mouseup', handleMouseUp);
    };

    const handleDoubleClick = () => {
        let defaultVal = (param.min_value + param.max_value) / 2;
        const nameLower = param.name.toLowerCase();
        if (nameLower.includes('volume') || nameLower.includes('gain')) {
            if (param.min_value <= 0 && param.max_value >= 0) {
                defaultVal = 0.0;
            }
        } else if (nameLower.includes('pan')) {
            defaultVal = 0.0;
        } else if (nameLower.includes('width')) {
            defaultVal = 1.0;
        }
        onChange(param.id, defaultVal);
    };

    const displayValue = (param.value ?? 0).toFixed(2);

    // Knob rotation: -135deg to +135deg (total 270)
    const rotation = -135 + (normalized * 270);

    return (
        <div
            className="param-knob-wrapper"
            title={param.name}
            style={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                gap: '4px',
                width: size + 10
            }}
        >
            <div
                className="param-knob"
                onMouseDown={handleMouseDown}
                onDoubleClick={handleDoubleClick}
                style={{
                    width: size,
                    height: size,
                    borderRadius: '50%',
                    background: '#222',
                    border: '1px solid #444',
                    position: 'relative',
                    cursor: 'ns-resize',
                    boxShadow: 'inset 0 0 10px #000'
                }}
            >
                {/* Indicator Line */}
                <div
                    style={{
                        position: 'absolute',
                        top: '50%', left: '50%',
                        width: '50%', height: '2px',
                        background: color,
                        transformOrigin: 'left center',
                        transform: `translate(0, -50%) rotate(${rotation}deg)`,
                        borderRadius: '1px',
                        boxShadow: `0 0 5px ${color}`
                    }}
                />
            </div>
            <div style={{ fontSize: '9px', color: '#aaa', overflow: 'hidden', whiteSpace: 'nowrap', textOverflow: 'ellipsis', maxWidth: '100%' }}>
                {param.name.toUpperCase()}
            </div>
            <div style={{ fontSize: '10px', color: color, fontFamily: 'monospace' }}>
                {displayValue}
            </div>
        </div>
    );
};
