import React, { useState, useEffect, useRef } from 'react';

interface DriveKnobProps {
    value: number; // 0.0 to 1.0
    onChange: (val: number) => void;
    size?: number;
}

export const DriveKnob: React.FC<DriveKnobProps> = ({ value, onChange, size = 40 }) => {
    const [isDragging, setIsDragging] = useState(false);
    const startY = useRef(0);
    const startVal = useRef(0);

    const handleMouseDown = (e: React.MouseEvent) => {
        setIsDragging(true);
        startY.current = e.clientY;
        startVal.current = value;
        document.body.style.cursor = 'ns-resize';
        window.addEventListener('mousemove', handleMouseMove);
        window.addEventListener('mouseup', handleMouseUp);
    };

    const handleMouseMove = (e: MouseEvent) => {
        const dy = startY.current - e.clientY;
        const speed = 0.005;
        let newVal = startVal.current + dy * speed;
        newVal = Math.max(0, Math.min(1, newVal));
        onChange(newVal);
    };

    const handleMouseUp = () => {
        setIsDragging(false);
        document.body.style.cursor = 'default';
        window.removeEventListener('mousemove', handleMouseMove);
        window.removeEventListener('mouseup', handleMouseUp);
    };

    const handleDoubleClick = () => {
        onChange(0.0);
    };

    // Calculate Thermal Color
    // 0.0: #00e5ff (Cyber Blue)
    // 0.5: #ff9d00 (Orange)
    // 0.8: #ff0055 (Red)
    // 1.0: #ffffff (White Hot)

    const getThermalColor = (v: number) => {
        if (v < 0.5) {
            // Blue to Orange
            const t = v * 2;
            // Interp
            return `hsl(${190 - t * 150}, 100%, 50%)`; // 190 -> 40
        } else if (v < 0.8) {
            // Orange to Red
            const t = (v - 0.5) / 0.3;
            return `hsl(${40 - t * 45}, 100%, 50%)`; // 40 -> -5 (355)
        } else {
            // Red to White
            const t = (v - 0.8) / 0.2;
            const l = 50 + t * 50;
            return `hsl(355, 100%, ${l}%)`;
        }
    };

    const color = getThermalColor(value);
    const glow = value > 0.0 ? `0 0 ${value * 15}px ${color}` : 'none';

    return (
        <div
            className="drive-knob-container"
            style={{
                width: size,
                height: size + 15,
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                gap: '2px'
            }}
        >
            <div
                className="drive-knob"
                onMouseDown={handleMouseDown}
                onDoubleClick={handleDoubleClick}
                style={{
                    width: size,
                    height: size,
                    borderRadius: '50%',
                    background: `conic-gradient(${color} ${value * 270}deg, #222 ${value * 270}deg 360deg)`,
                    transform: 'rotate(-135deg)',
                    boxShadow: glow,
                    cursor: 'ns-resize',
                    position: 'relative',
                    border: '1px solid #444'
                }}
            >
                {/* Inner Cap */}
                <div style={{
                    position: 'absolute',
                    top: '10%', left: '10%',
                    width: '80%', height: '80%',
                    borderRadius: '50%',
                    background: '#1a1a1a',
                    display: 'flex',
                    justifyContent: 'center',
                    alignItems: 'center'
                }}>
                    <div className="knob-label" style={{
                        transform: `rotate(${135}deg)`, // Counter-rotate text
                        fontSize: '9px',
                        color: value > 0.0 ? color : '#666',
                        fontWeight: 'bold',
                        pointerEvents: 'none',
                        userSelect: 'none'
                    }}>
                        {Math.floor(value * 100)}%
                    </div>
                </div>
            </div>
            <span style={{ fontSize: '9px', color: '#888' }}>DRIVE</span>
        </div>
    );
};
