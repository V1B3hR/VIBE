import React, { useRef, useEffect, useState } from 'react';

interface MicroScopeProps {
    pan: number;   // -1.0 (L) to 1.0 (R)
    widthVal: number; // 0.0 (Mono) to 2.0 (Extra Wide)
    onPanChange: (val: number) => void;
    onWidthChange: (val: number) => void;
    size?: number; // default 60
}

export const MicroScope: React.FC<MicroScopeProps> = ({ pan, widthVal, onPanChange, onWidthChange, size = 60 }) => {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const [isDragging, setIsDragging] = useState(false);

    // Drag state
    const startX = useRef(0);
    const startY = useRef(0);
    const startPan = useRef(0);
    const startWidth = useRef(0);

    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        // Clear
        ctx.clearRect(0, 0, size, size); // Transparent or glass bg handled by parent

        // Background Grid (Polar/Vector)
        ctx.strokeStyle = '#333';
        ctx.lineWidth = 1;
        ctx.beginPath();
        // Crosshair
        ctx.moveTo(size / 2, 0); ctx.lineTo(size / 2, size);
        ctx.moveTo(0, size / 2); ctx.lineTo(size, size / 2);
        // Diagonal (L=R)
        ctx.moveTo(0, size); ctx.lineTo(size, 0); // /
        ctx.moveTo(0, 0); ctx.lineTo(size, size); // \
        ctx.stroke();

        // Draw "Field" Representation
        // Center position based on Pan
        // Pan -1 maps to X=0, +1 maps to X=size
        const cx = (pan + 1) / 2 * size;
        const cy = size / 2; // Fixed Y center for position? Or Up/Down means Width?

        // In this control, Drag Y changes WIDTH.
        // Visually, Width changes the shape's spread.

        // Draw ellipse
        // Width 0 -> Line/Dot. Width 1 -> Circle. Width 2 -> Wide Ellipse.
        let rx = (widthVal / 2.0) * (size / 2); // Max width = full width
        let ry = (size / 4); // Fixed height factor or dynamic? 
        // Let's make it look like a cloud.

        // Clamp rx simple
        rx = Math.max(2, rx);

        ctx.fillStyle = 'rgba(0, 229, 255, 0.3)'; // Cyan tint
        ctx.strokeStyle = '#00e5ff';
        ctx.lineWidth = 1.5;

        ctx.beginPath();
        ctx.ellipse(cx, cy, rx, ry, 0, 0, Math.PI * 2);
        ctx.fill();
        ctx.stroke();

        // Draw Center Dot
        ctx.fillStyle = '#fff';
        ctx.beginPath();
        ctx.arc(cx, cy, 3, 0, Math.PI * 2);
        ctx.fill();

        // Text Feedback
        if (isDragging) {
            ctx.fillStyle = '#fff';
            ctx.font = '9px Inter';
            ctx.textAlign = 'center';
            ctx.fillText(`P:${pan.toFixed(2)} W:${widthVal.toFixed(2)}`, size / 2, size - 4);
        }

    }, [pan, widthVal, size, isDragging]);

    const handleMouseDown = (e: React.MouseEvent) => {
        setIsDragging(true);
        startX.current = e.clientX;
        startY.current = e.clientY;
        startPan.current = pan;
        startWidth.current = widthVal;

        document.body.style.cursor = 'move';
        window.addEventListener('mousemove', handleMouseMove);
        window.addEventListener('mouseup', handleMouseUp);
    };

    const handleMouseMove = (e: MouseEvent) => {
        const dx = e.clientX - startX.current;
        const dy = startY.current - e.clientY; // Drag UP increases width

        // Sensitivity
        const panSense = 0.01;
        const widthSense = 0.01;

        let newPan = startPan.current + dx * panSense;
        newPan = Math.max(-1, Math.min(1, newPan));

        let newWidth = startWidth.current + dy * widthSense;
        newWidth = Math.max(0, Math.min(2, newWidth));

        onPanChange(newPan);
        onWidthChange(newWidth);
    };

    const handleMouseUp = () => {
        setIsDragging(false);
        document.body.style.cursor = 'default';
        window.removeEventListener('mousemove', handleMouseMove);
        window.removeEventListener('mouseup', handleMouseUp);
    };

    const handleDoubleClick = () => {
        onPanChange(0.0);
        onWidthChange(1.0);
    };

    return (
        <div className="micro-scope-container" style={{ width: size, height: 'auto', display: 'flex', flexDirection: 'column', alignItems: 'center' }}>
            <canvas
                ref={canvasRef}
                width={size}
                height={size}
                onMouseDown={handleMouseDown}
                onDoubleClick={handleDoubleClick}
                style={{
                    borderRadius: '50%',
                    background: '#111',
                    border: '1px solid #333',
                    cursor: 'move',
                    boxShadow: 'inset 0 0 10px rgba(0,0,0,0.8)'
                }}
            />
            <span style={{ fontSize: '9px', color: '#666', marginTop: '2px' }}>VECTOR</span>
        </div>
    );
};
