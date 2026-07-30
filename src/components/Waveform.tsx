import { useRef, useEffect } from "react";

interface WaveformProps {
    peaks: number[][]; // Multi-level peaks (MIP-maps)
    color?: string;
}

export const Waveform = ({ peaks, color = "#6366f1" }: WaveformProps) => {
    const canvasRef = useRef<HTMLCanvasElement>(null);

    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas || !peaks || peaks.length === 0) return;

        const ctx = canvas.getContext("2d");
        if (!ctx) return;

        const dpr = window.devicePixelRatio || 1;
        const rect = canvas.getBoundingClientRect();

        // Ensure crisp rendering on high-DPI displays
        canvas.width = rect.width * dpr;
        canvas.height = rect.height * dpr;
        ctx.scale(dpr, dpr);

        const width = rect.width;
        const height = rect.height;
        ctx.clearRect(0, 0, width, height);

        // Select best peak level based on width
        // We want at least 1 peak per pixel if possible, or the best available
        let bestLevel = 0;
        for (let i = 0; i < peaks.length; i++) {
            if (peaks[i].length >= width) {
                bestLevel = i;
            } else {
                break;
            }
        }

        const currentPeaks = peaks[bestLevel];
        if (!currentPeaks) return;

        const step = width / currentPeaks.length;
        const centerY = height / 2;

        ctx.beginPath();
        const gradient = ctx.createLinearGradient(0, 0, 0, height);
        gradient.addColorStop(0, color);
        gradient.addColorStop(0.5, "#fff"); // Center highlight
        gradient.addColorStop(1, color);

        ctx.strokeStyle = gradient;
        ctx.lineWidth = 1;

        for (let i = 0; i < currentPeaks.length; i++) {
            const x = i * step;
            const peak = currentPeaks[i];
            const yOffset = peak * (height / 2) * 0.95;

            ctx.moveTo(x, centerY - yOffset);
            ctx.lineTo(x, centerY + yOffset);
        }

        ctx.stroke();
    }, [peaks, color]);

    return (
        <canvas
            ref={canvasRef}
            style={{ width: "100%", height: "100%", display: "block", opacity: 0.9 }}
        />
    );
};
