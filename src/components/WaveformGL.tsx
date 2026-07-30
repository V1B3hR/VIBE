import { useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface WaveformGLProps {
    clipId: string;
    width: number;
    height: number;
    color: string;
    // Viewport
    startSample: number;
    endSample: number;
    totalSamples: number;
    pixelsPerSample?: number; // Smart LOD
    verticalScale?: number;   // Visual magnification (1.0 = normal)
    displayMode?: 'Bars' | 'Oscilloscope' | 'Rectified' | 'Spectrum';
}

// Module-level cache to eliminate redundant Tauri IPC calls during horizontal scrolling
const LOD_CACHE = new Map<string, number[]>();

// ─── Shaders ───────────────────────────────────────────────────────────────

// Bar (LOD) shader — renders instanced min/max/rms columns
const VS_BAR = `#version 300 es
precision highp float;
layout(location=0) in vec2 aPosition;
layout(location=1) in float aMin;
layout(location=2) in float aMax;
layout(location=3) in float aRms;
uniform vec2 uScale;
uniform vec2 uOffset;
uniform int uDisplayMode; // 0=Bars, 1=Rectified, 2=Spectrum
out float vRms;
out float vY;
void main() {
    float id = float(gl_InstanceID);
    float x_center = (id + uOffset.x) * uScale.x - 1.0;
    float w = uScale.x;
    float x = x_center + aPosition.x * w;
    
    float y = 0.0;
    if (uDisplayMode == 1) {
        // Rectified: positive peaks starting from bottom (0.0)
        y = (0.0 + aMax * aPosition.y) * uOffset.y;
    } else if (uDisplayMode == 2) {
        // Spectrum: frequency simulation (RMS-based heat columns starting from 0)
        y = (0.0 + (aRms * 1.6) * aPosition.y) * uOffset.y;
    } else {
        // Bars (standard bipolar waveform)
        y = (aMin + (aMax - aMin) * aPosition.y) * uOffset.y;
    }
    
    vRms = aRms;
    vY = y;
    gl_Position = vec4(x, y, 0.0, 1.0);
}`;

const FS_BAR = `#version 300 es
precision highp float;
in float vRms;
in float vY;
uniform vec4 uColor;
uniform int uDisplayMode;
out vec4 fragColor;
void main() {
    float energy = vRms;
    vec3 col = uColor.rgb + (energy * 0.4);
    if (uDisplayMode == 2) {
        // Heatmap colors for mock spectrum: low freq/amplitude (blue/cyan) -> high (red/orange)
        col = mix(vec3(0.0, 0.8, 0.8), vec3(1.0, 0.2, 0.1), clamp(energy * 2.0, 0.0, 1.0));
    }
    float centerGlow = 1.0 - abs(vY) * 0.25;
    fragColor = vec4(col * centerGlow, uColor.a * (0.75 + energy * 0.25));
}`;

// Analog (raw samples) shader
const VS_ANALOG = `#version 300 es
precision highp float;
layout(location=0) in float aValue;
uniform vec2 uScale;
uniform vec2 uOffset;
uniform int uDisplayMode; // 0=Oscilloscope, 1=Rectified, 2=Spectrum
out float vY;
out float vRaw;
void main() {
    float id = float(gl_VertexID);
    float x = (id + uOffset.x) * uScale.x - 1.0;
    
    float y = 0.0;
    if (uDisplayMode == 1) {
        y = abs(aValue) * uOffset.y;
    } else {
        y = aValue * uOffset.y;
    }
    
    vY = y;
    vRaw = aValue;
    gl_Position = vec4(x, y, 0.0, 1.0);
    gl_PointSize = 3.0;
}`;

const FS_ANALOG = `#version 300 es
precision highp float;
in float vY;
in float vRaw;
uniform vec4 uColor;
out vec4 fragColor;
void main() {
    float alpha = 0.9 + 0.1 * abs(vRaw);
    fragColor = vec4(uColor.rgb, uColor.a * alpha);
}`;

// ─── LOD picker ────────────────────────────────────────────────────────────
// Returns: -1=analog(raw), 0=LOD0(16samp/pt), 1=LOD1(128), 2=LOD2(2048), 3=LOD3(65536)
const pickLod = (pps: number): number => {
    if (pps > 0.08) return -1;       // individual samples visible (>~4 px/sample)
    if (pps > 0.008) return 0;        // LOD0: 10-50ms zoom — HQ detail
    if (pps > 0.0008) return 1;       // LOD1: 100ms–1s
    if (pps > 0.00001) return 2;      // LOD2: 1s–20s
    return 3;                          // LOD3: full song overview
};

// samples per LOD point for each level
const SAMPLES_PER_PT = [16, 128, 2048, 65536];

// ─── Component ─────────────────────────────────────────────────────────────
export const WaveformGL = ({
    clipId,
    width,
    height,
    color: _color,
    startSample,
    endSample,
    totalSamples: _totalSamples,
    pixelsPerSample = 0.0005,
    verticalScale = 1.0,
    displayMode: defaultDisplayMode = 'Bars'
}: WaveformGLProps) => {
    const displayMode = localStorage.getItem(`vibe/clip-mode/${clipId}`) as any || defaultDisplayMode;
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const glRef = useRef<WebGL2RenderingContext | null>(null);
    const progBarRef = useRef<WebGLProgram | null>(null);
    const progAnalogRef = useRef<WebGLProgram | null>(null);
    const bufferRef = useRef<WebGLBuffer | null>(null);
    const pointsRef = useRef<number>(0);
    const lodRef = useRef<number>(-99);
    const isAnalogRef = useRef<boolean>(false);
    const dprRef = useRef<number>(1);

    const parseHex = (hex: string) => {
        const h = hex.replace('#', '');
        const r = parseInt(h.substring(0, 2), 16) / 255 || 0.5;
        const g = parseInt(h.substring(2, 4), 16) / 255 || 0.8;
        const b = parseInt(h.substring(4, 6), 16) / 255 || 1.0;
        return { r, g, b };
    };
    const color = parseHex(_color);

    // ── Init GL ──────────────────────────────────────────────────────────
    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;

        const dpr = window.devicePixelRatio || 1;
        dprRef.current = dpr;
        canvas.width = Math.round(width * dpr);
        canvas.height = Math.round(height * dpr);

        const gl = canvas.getContext('webgl2', {
            alpha: true,
            antialias: true,
            powerPreference: 'high-performance',
        });
        if (!gl) return;
        glRef.current = gl;

        const compile = (vsSrc: string, fsSrc: string) => {
            const vs = gl.createShader(gl.VERTEX_SHADER)!;
            gl.shaderSource(vs, vsSrc);
            gl.compileShader(vs);
            if (!gl.getShaderParameter(vs, gl.COMPILE_STATUS)) console.error(gl.getShaderInfoLog(vs));

            const fs = gl.createShader(gl.FRAGMENT_SHADER)!;
            gl.shaderSource(fs, fsSrc);
            gl.compileShader(fs);
            if (!gl.getShaderParameter(fs, gl.COMPILE_STATUS)) console.error(gl.getShaderInfoLog(fs));

            const p = gl.createProgram()!;
            gl.attachShader(p, vs);
            gl.attachShader(p, fs);
            gl.linkProgram(p);
            return p;
        };

        progBarRef.current = compile(VS_BAR, FS_BAR);
        progAnalogRef.current = compile(VS_ANALOG, FS_ANALOG);

        // Quad buffer for instanced bars
        const quadData = new Float32Array([0, 0, 1, 0, 0, 1, 1, 1]);
        const quadBuf = gl.createBuffer();
        gl.bindBuffer(gl.ARRAY_BUFFER, quadBuf);
        gl.bufferData(gl.ARRAY_BUFFER, quadData, gl.STATIC_DRAW);

        // Instance/analog buffer
        const instBuf = gl.createBuffer();
        gl.bindBuffer(gl.ARRAY_BUFFER, instBuf);
        bufferRef.current = instBuf;
    }, []);

    // ── DPR canvas resize ─────────────────────────────────────────────────
    useEffect(() => {
        const canvas = canvasRef.current;
        const gl = glRef.current;
        if (!canvas || !gl) return;
        const dpr = window.devicePixelRatio || 1;
        dprRef.current = dpr;
        const pw = Math.round(width * dpr);
        const ph = Math.round(height * dpr);
        if (canvas.width !== pw || canvas.height !== ph) {
            canvas.width = pw;
            canvas.height = ph;
        }
    }, [width, height]);

    // ── Fetch & Upload Data ───────────────────────────────────────────────
    const selectedLod = pickLod(pixelsPerSample);

    useEffect(() => {
        if (!clipId) return;

        const fetchData = async () => {
            try {
                const gl = glRef.current;
                if (!gl) return;

                if (selectedLod === -1) {
                    // ── Analog mode: raw sample fetch ──────────────────
                    isAnalogRef.current = true;
                    // Small margin for smooth line at edges
                    const margin = Math.min(2400, Math.ceil(48 / (pixelsPerSample || 1e-6)));
                    const start = Math.max(0, Math.floor(startSample - margin));
                    const end = Math.ceil(endSample + margin);

                    const samples: number[] = await invoke('get_raw_samples', {
                        clipId,
                        startSample: start,
                        endSample: end,
                    });
                    if (!samples || samples.length === 0) return;

                    const data = new Float32Array(samples);
                    pointsRef.current = data.length;
                    lodRef.current = -1;

                    gl.bindBuffer(gl.ARRAY_BUFFER, bufferRef.current);
                    gl.bufferData(gl.ARRAY_BUFFER, data, gl.DYNAMIC_DRAW);
                    gl.enableVertexAttribArray(0);
                    gl.vertexAttribPointer(0, 1, gl.FLOAT, false, 0, 0);
                    gl.vertexAttribDivisor(0, 0);

                } else {
                    // ── Bar LOD mode ───────────────────────────────────
                    isAnalogRef.current = false;

                    // Only re-fetch when LOD level actually changes
                    if (lodRef.current === selectedLod && pointsRef.current > 0) {
                        requestAnimationFrame(render);
                        return;
                    }
                    lodRef.current = selectedLod;

                    const cacheKey = `${clipId}_${selectedLod}`;
                    let data = LOD_CACHE.get(cacheKey);

                    if (!data) {
                        data = await invoke('get_waveform_chunk', { clipId, lodLevel: selectedLod });
                        if (data) LOD_CACHE.set(cacheKey, data as number[]);
                    }
                    if (!data) return;

                    const u8 = new Uint8Array(data);
                    const u16 = new Uint16Array(u8.buffer);
                    pointsRef.current = u16.length / 3; // 3 half-floats per point: min, max, rms

                    gl.bindBuffer(gl.ARRAY_BUFFER, bufferRef.current);
                    gl.bufferData(gl.ARRAY_BUFFER, u16, gl.STATIC_DRAW);

                    gl.enableVertexAttribArray(1);
                    gl.vertexAttribPointer(1, 1, gl.HALF_FLOAT, false, 6, 0);
                    gl.vertexAttribDivisor(1, 1);

                    gl.enableVertexAttribArray(2);
                    gl.vertexAttribPointer(2, 1, gl.HALF_FLOAT, false, 6, 2);
                    gl.vertexAttribDivisor(2, 1);

                    gl.enableVertexAttribArray(3);
                    gl.vertexAttribPointer(3, 1, gl.HALF_FLOAT, false, 6, 4);
                    gl.vertexAttribDivisor(3, 1);
                }

                requestAnimationFrame(render);
            } catch (e) {
                console.error('Waveform fetch failed', e);
            }
        };

        fetchData();
    }, [
        clipId,
        selectedLod,
        // For analog: re-fetch every 4800 samples (~100ms scroll window)
        selectedLod === -1 ? Math.floor(startSample / 4800) : 0,
        selectedLod === -1 ? Math.floor(endSample / 4800) : 0,
    ]);

    // ── Render ────────────────────────────────────────────────────────────
    const render = () => {
        const gl = glRef.current;
        if (!gl || !progBarRef.current || !progAnalogRef.current) return;

        const dpr = dprRef.current;
        gl.viewport(0, 0, Math.round(width * dpr), Math.round(height * dpr));
        gl.clearColor(0, 0, 0, 0);
        gl.clear(gl.COLOR_BUFFER_BIT);

        if (pointsRef.current === 0) return;

        const modeInt = displayMode === 'Rectified' ? 1 : displayMode === 'Spectrum' ? 2 : 0;

        if (isAnalogRef.current) {
            gl.useProgram(progAnalogRef.current);
            const margin = Math.min(2400, Math.ceil(48 / (pixelsPerSample || 1e-6)));
            const start = Math.max(0, Math.floor(startSample - margin));
            const visibleSamples = endSample - startSample;

            const scaleX = 2.0 / (visibleSamples || 1);
            const offsetX = -(startSample - start);

            gl.uniform2f(gl.getUniformLocation(progAnalogRef.current, 'uScale'), scaleX, 1.0);
            gl.uniform2f(gl.getUniformLocation(progAnalogRef.current, 'uOffset'), offsetX, verticalScale);
            gl.uniform4f(gl.getUniformLocation(progAnalogRef.current, 'uColor'), color.r, color.g, color.b, 1.0);
            gl.uniform1i(gl.getUniformLocation(progAnalogRef.current, 'uDisplayMode'), modeInt);

            gl.drawArrays(gl.LINE_STRIP, 0, pointsRef.current);
            // Dot overlay only at very high zoom (individual samples clearly visible)
            if (pixelsPerSample > 0.5) {
                gl.drawArrays(gl.POINTS, 0, pointsRef.current);
            }

        } else {
            gl.useProgram(progBarRef.current);

            const spp = SAMPLES_PER_PT[Math.max(0, lodRef.current)] ?? 128;
            const visiblePts = (endSample - startSample) / spp;
            const startPt = startSample / spp;

            gl.uniform2f(gl.getUniformLocation(progBarRef.current, 'uScale'), 2.0 / (visiblePts || 1), 1.0);
            gl.uniform2f(gl.getUniformLocation(progBarRef.current, 'uOffset'), -startPt, verticalScale);
            gl.uniform4f(gl.getUniformLocation(progBarRef.current, 'uColor'), color.r, color.g, color.b, 1.0);
            gl.uniform1i(gl.getUniformLocation(progBarRef.current, 'uDisplayMode'), modeInt);

            gl.drawArraysInstanced(gl.TRIANGLE_STRIP, 0, 4, pointsRef.current);
        }
    };

    useEffect(() => {
        requestAnimationFrame(render);
    }, [startSample, endSample, width, height, pixelsPerSample, verticalScale]);

    // CSS logical size; canvas physical buffer = logical × DPR
    return <canvas ref={canvasRef} style={{ width, height, display: 'block' }} />;
};
