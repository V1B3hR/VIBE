import React, { useRef, useEffect } from 'react';
import { aiAssistant } from '../services/AiAssistService';

interface NanoEqProps {
    trackId: number; // For identifying context
    processorId: string;
    params: any[]; // Parameter list
    width?: number;
    height?: number;
    onClick?: () => void;
}

export const NanoEq: React.FC<NanoEqProps> = ({ params, width = 60, height = 40, onClick, processorId }) => {
    const canvasRef = useRef<HTMLCanvasElement>(null);

    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        // Clear
        ctx.fillStyle = '#1a1a1a';
        ctx.fillRect(0, 0, width, height);

        // Grid (subtle)
        ctx.strokeStyle = '#333';
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(width / 2, 0); ctx.lineTo(width / 2, height);
        ctx.moveTo(0, height / 2); ctx.lineTo(width, height / 2);
        ctx.stroke();

        // Draw EQ Curve (Approximation)
        // We need to parse params. Usually: LowGain, LowFreq, MidGain, MidFreq, HighGain, HighFreq
        // Assuming Console EQ structure.
        let lowGain = 0, midGain = 0, highGain = 0;
        let lowFreq = 100, midFreq = 1000, highFreq = 5000;

        params.forEach(p => {
            if (p.name.includes("Low Gain")) lowGain = p.value;
            if (p.name.includes("Mid Gain")) midGain = p.value;
            if (p.name.includes("High Gain")) highGain = p.value;
            if (p.name.includes("Low Freq")) lowFreq = p.value;
            if (p.name.includes("Mid Freq")) midFreq = p.value;
            if (p.name.includes("High Freq")) highFreq = p.value;
        });

        // Function to compute magnitude at freq
        const getMag = (f: number) => {
            // Simplified shelf/peak logic for visualization
            // 0dB is at height/2
            let db = 0;
            // Low Shelf
            if (f < lowFreq) db += lowGain;
            else if (f < lowFreq * 2) db += lowGain * (1 - (f - lowFreq) / lowFreq); // linear falloff approx

            // High Shelf
            if (f > highFreq) db += highGain;

            // Mid Bell (Simplified)
            let midQ = 0.5; // Fixed Q visual
            let dist = Math.abs(Math.log10(f) - Math.log10(midFreq));
            if (dist < midQ) {
                db += midGain * (1.0 - dist / midQ);
            }

            return db;
        };

        ctx.strokeStyle = '#00ff9d'; // Cyberpunk Green
        ctx.lineWidth = 2;
        ctx.shadowColor = '#00ff9d';
        ctx.shadowBlur = 4;
        ctx.beginPath();

        for (let x = 0; x < width; x++) {
            // Log scale x
            let t = x / width;
            let f = 20 * Math.pow(1000, t); // 20Hz to 20kHz
            let db = getMag(f);

            // Map dB to Y. +/- 15dB range
            let y = height / 2 - (db / 15.0) * (height / 2);
            if (x === 0) ctx.moveTo(x, y);
            else ctx.lineTo(x, y);
        }
        ctx.stroke();

        // Border
        ctx.strokeStyle = '#444';
        ctx.lineWidth = 1;
        ctx.shadowBlur = 0;
        ctx.strokeRect(0, 0, width, height);

    }, [params, width, height]);

    return (
        <canvas
            ref={canvasRef}
            width={width}
            height={height}
            onClick={onClick}
            onMouseEnter={() => aiAssistant.setFocusedElement('NanoEQ', processorId)}
            onMouseLeave={() => aiAssistant.setFocusedElement(null, null)}
            onMouseMove={(e) => {
                const rect = canvasRef.current?.getBoundingClientRect();
                if (rect) {
                    const x = e.clientX - rect.left;
                    // simple visual approx: left = low, mid = mid, right = high
                    let paramFocus = 'Gain';
                    if (x < width / 3) paramFocus = 'Low Band';
                    else if (x > width * 0.66) paramFocus = 'High Band';
                    else paramFocus = 'Mid Band';
                    aiAssistant.setFocusedElement('NanoEQ', paramFocus);
                }
            }}
            style={{ cursor: 'pointer', borderRadius: '4px' }}
        />
    );
};
