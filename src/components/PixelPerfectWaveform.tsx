import { useRef, useEffect, memo } from "react";

interface WebGLWaveformProps {
    peaks: number[][]; // Multi-level peaks (MIP-maps)
    color?: string;
}

const PixelPerfectWaveformComponent = ({ peaks, color = "#6366f1" }: WebGLWaveformProps) => {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const glRef = useRef<WebGL2RenderingContext | null>(null);
    const programRef = useRef<WebGLProgram | null>(null);

    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;

        const gl = canvas.getContext("webgl2", { antialias: true, alpha: true });
        if (!gl) return;
        glRef.current = gl;

        const vsSource = `#version 300 es
            layout(location = 0) in vec2 a_position;
            uniform vec2 u_resolution;
            void main() {
                // Convert from pixel space to clip space
                vec2 clipSpace = (a_position / u_resolution) * 2.0 - 1.0;
                gl_Position = vec4(clipSpace * vec2(1, -1), 0, 1);
            }
        `;

        const fsSource = `#version 300 es
            precision highp float;
            uniform vec4 u_color;
            out vec4 outColor;
            void main() {
                outColor = u_color;
            }
        `;

        const vs = gl.createShader(gl.VERTEX_SHADER)!;
        gl.shaderSource(vs, vsSource);
        gl.compileShader(vs);

        const fs = gl.createShader(gl.FRAGMENT_SHADER)!;
        gl.shaderSource(fs, fsSource);
        gl.compileShader(fs);

        const program = gl.createProgram()!;
        gl.attachShader(program, vs);
        gl.attachShader(program, fs);
        gl.linkProgram(program);
        programRef.current = program;

        return () => {
            gl.deleteProgram(program);
            gl.deleteShader(vs);
            gl.deleteShader(fs);
        };
    }, []);

    useEffect(() => {
        const gl = glRef.current;
        const program = programRef.current;
        if (!gl || !program || !peaks || peaks.length === 0) return;

        const dpr = window.devicePixelRatio || 1;
        const rect = canvasRef.current!.getBoundingClientRect();
        const width = rect.width;
        const height = rect.height;

        canvasRef.current!.width = width * dpr;
        canvasRef.current!.height = height * dpr;
        gl.viewport(0, 0, width * dpr, height * dpr);

        // Select optimal peak level (MIP-maps)
        // peaks are [1k, 10k, 100k]. We want the smallest level that has >= width points.
        let bestLevel = peaks.length - 1; // Default to highest resolution
        for (let i = 0; i < peaks.length; i++) {
            if (peaks[i].length >= width) {
                bestLevel = i;
                break;
            }
        }

        const data = peaks[bestLevel];
        const step = width / data.length;
        const centerY = height / 2;

        // Create buffer for drawing vertical lines for each peak
        const vertices = new Float32Array(data.length * 4);
        for (let i = 0; i < data.length; i++) {
            const x = i * step;
            const amp = data[i] * (height / 2) * 0.9;

            vertices[i * 4 + 0] = x;
            vertices[i * 4 + 1] = centerY - amp;
            vertices[i * 4 + 2] = x;
            vertices[i * 4 + 3] = centerY + amp;
        }

        const buffer = gl.createBuffer();
        gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
        gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.STREAM_DRAW);

        gl.useProgram(program);

        const posLoc = gl.getAttribLocation(program, "a_position");
        const resLoc = gl.getUniformLocation(program, "u_resolution");
        const colorLoc = gl.getUniformLocation(program, "u_color");

        gl.enableVertexAttribArray(posLoc);
        gl.vertexAttribPointer(posLoc, 2, gl.FLOAT, false, 0, 0);

        gl.uniform2f(resLoc, width, height);

        // Color
        const r = parseInt(color.slice(1, 3), 16) / 255;
        const g = parseInt(color.slice(3, 5), 16) / 255;
        const b = parseInt(color.slice(5, 7), 16) / 255;
        gl.uniform4f(colorLoc, r, g, b, 1.0);

        gl.clearColor(0, 0, 0, 0);
        gl.clear(gl.COLOR_BUFFER_BIT);

        gl.drawArrays(gl.LINES, 0, data.length * 2);

        gl.deleteBuffer(buffer);
    }, [peaks, color]);

    return (
        <canvas
            ref={canvasRef}
            style={{ width: "100%", height: "100%", display: "block" }}
        />
    );
};

// Memoize with custom comparison - only re-render if peaks content actually changed
export const PixelPerfectWaveform = memo(PixelPerfectWaveformComponent, (prevProps, nextProps) => {
    // If color changed, re-render
    if (prevProps.color !== nextProps.color) return false;

    // If peaks array structure changed (length or nested lengths), re-render
    if (prevProps.peaks.length !== nextProps.peaks.length) return false;

    // Deep comparison would be expensive, but we can check first level length
    for (let i = 0; i < prevProps.peaks.length; i++) {
        if (prevProps.peaks[i].length !== nextProps.peaks[i].length) return false;
    }

    // Props are equal, skip re-render
    return true;
});
