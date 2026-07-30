import React, { useEffect, useRef, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './EqCanvas.css';

interface EqBand {
    id: string;
    enabled: boolean;
    filter_type: 'LowPass' | 'HighPass' | 'Bell' | 'LowShelf' | 'HighShelf' | 'Notch';
    freq: number;
    gain_db: number;
    q: number;
    mode: 'Stereo' | 'Left' | 'Right' | 'Mid' | 'Side';
    solo: boolean;
}

interface EqCanvasProps {
    trackId: number;
    processorId: string;
    onUpdateBands?: (bands: EqBand[]) => void;
}

const FREQ_MIN = 20;
const FREQ_MAX = 20000;
const GAIN_MIN = -24;
const GAIN_MAX = 24;

const freqToX = (freq: number, width: number) => {
    const logMin = Math.log10(FREQ_MIN);
    const logMax = Math.log10(FREQ_MAX);
    const logFreq = Math.log10(freq);
    return ((logFreq - logMin) / (logMax - logMin)) * width;
};

const xToFreq = (x: number, width: number) => {
    const logMin = Math.log10(FREQ_MIN);
    const logMax = Math.log10(FREQ_MAX);
    const logFreq = logMin + (x / width) * (logMax - logMin);
    return Math.pow(10, logFreq);
};

const gainToY = (gain: number, height: number) => {
    const range = GAIN_MAX - GAIN_MIN;
    return height - ((gain - GAIN_MIN) / range) * height;
};

const yToGain = (y: number, height: number) => {
    const range = GAIN_MAX - GAIN_MIN;
    return GAIN_MIN + ((height - y) / height) * range;
};

const NOTE_NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];
const freqToNote = (freq: number): string => {
    if (freq < 10) return '';
    const noteNum = 12 * (Math.log2(freq / 440)) + 69;
    const note = Math.round(noteNum);
    const octave = Math.floor(note / 12) - 1;
    const name = NOTE_NAMES[note % 12];
    return `${name}${octave}`;
};

export const EqCanvas: React.FC<EqCanvasProps> = ({ trackId, processorId }) => {
    const containerRef = useRef<HTMLDivElement>(null);
    const gridCanvasRef = useRef<HTMLCanvasElement>(null);
    const spectrumCanvasRef = useRef<HTMLCanvasElement>(null);
    const curveCanvasRef = useRef<HTMLCanvasElement>(null);

    // WebGL refs for GPU Spectrum
    const glRef = useRef<WebGLRenderingContext | null>(null);
    const programRef = useRef<WebGLProgram | null>(null);
    const textureRef = useRef<WebGLTexture | null>(null);

    const [bands, setBands] = useState<EqBand[]>([]);
    const [selectedBandId, setSelectedBandId] = useState<string | null>(null);
    const [hoveredBandId, setHoveredBandId] = useState<string | null>(null);

    const [mousePos, setMousePos] = useState({ x: 0, y: 0 });
    const [isDragging, setIsDragging] = useState(false);

    const lastUpdateRef = useRef<number>(0);
    const updateBandBackend = useCallback((band: EqBand) => {
        const now = Date.now();
        // Throttle to 16ms (~60fps) to match monitor refresh and avoid IPC congestion
        if (now - lastUpdateRef.current < 16) return;

        lastUpdateRef.current = now;
        invoke('update_eq_band', {
            trackIdx: trackId,
            processorId: processorId,
            band: band
        }).catch(err => console.error("Failed to update EQ band:", err));
    }, [trackId, processorId]);

    // 0. Fetch Initial Bands
    useEffect(() => {
        invoke<EqBand[]>('get_eq_bands', { trackIdx: trackId, processorId: processorId })
            .then(b => {
                if (b && b.length > 0) {
                    setBands(b);
                }
            })
            .catch(err => console.error("Failed to fetch EQ bands:", err));
    }, [trackId, processorId]);

    // 1. Setup WebGL for Spectrum
    useEffect(() => {
        const canvas = spectrumCanvasRef.current;
        if (!canvas) return;

        // Initialize WebGL
        const gl = canvas.getContext("webgl", { alpha: true, premultipliedAlpha: false });
        if (!gl) {
            console.warn("WebGL not supported for EQ Spectrum");
            return;
        }
        glRef.current = gl;

        const vsSource = `
            attribute vec2 a_position;
            varying vec2 v_uv;
            void main() {
                v_uv = a_position * 0.5 + 0.5;
                gl_Position = vec4(a_position, 0.0, 1.0);
            }
        `;

        const fsSource = `
            precision mediump float;
            varying vec2 v_uv;
            uniform sampler2D u_tex;

            void main() {
                // Map linear v_uv.x to logarithmic frequency (20Hz to 20kHz)
                float logMin = log(20.0);
                float logMax = log(20000.0);
                float logFreq = logMin + v_uv.x * (logMax - logMin);
                float freq = exp(logFreq);
                
                // Map frequency to texture coordinate (linear FFT bins: 0 to 20kHz)
                float texCoordX = freq / 20000.0;
                
                if (texCoordX > 1.0) discard;

                // Sample magnitude
                float mag = texture2D(u_tex, vec2(texCoordX, 0.5)).a;
                
                // Convert DB magnitude to visual height (matching CPU formula)
                // CPU: h = (mag + 80) / 80 * height; y_viz = height - max(0, h * 0.9);
                // In UV coordinates (0 to 1, bottom to top):
                float h = max(0.0, (mag + 80.0) / 80.0);
                float targetY = h * 0.9;
                
                if (v_uv.y > targetY) {
                    discard;
                }

                // Dynamic Spectrum Fire Gradient
                vec3 colorBottom = vec3(0.2, 0.2, 0.6);   // rgba(50, 50, 150)
                vec3 colorMid = vec3(0.78, 0.2, 0.6);     // rgba(200, 50, 150)
                vec3 colorTop = vec3(1.0, 0.39, 0.2);     // rgba(255, 100, 50)
                
                vec3 color = mix(colorBottom, colorMid, smoothstep(0.0, 0.5, v_uv.y));
                color = mix(color, colorTop, smoothstep(0.5, 1.0, v_uv.y));
                
                // Opacity gradient (fades out at bottom)
                float alpha = mix(0.2, 0.6, v_uv.y);
                
                // Smooth top edge
                float edge = smoothstep(targetY, targetY - 0.02, v_uv.y);
                gl_FragColor = vec4(color * alpha * edge, alpha * edge);
            }
        `;

        const compileShader = (type: number, source: string) => {
            const shader = gl.createShader(type);
            if (!shader) return null;
            gl.shaderSource(shader, source);
            gl.compileShader(shader);
            return shader;
        };

        const vs = compileShader(gl.VERTEX_SHADER, vsSource);
        const fs = compileShader(gl.FRAGMENT_SHADER, fsSource);
        const program = gl.createProgram();
        if (!program || !vs || !fs) return;

        gl.attachShader(program, vs);
        gl.attachShader(program, fs);
        gl.linkProgram(program);
        gl.useProgram(program);
        programRef.current = program;

        // Quad vertices
        const vertices = new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]);
        const vbo = gl.createBuffer();
        gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
        gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.STATIC_DRAW);

        const posAttr = gl.getAttribLocation(program, "a_position");
        gl.enableVertexAttribArray(posAttr);
        gl.vertexAttribPointer(posAttr, 2, gl.FLOAT, false, 0, 0);

        // Texture setup
        const texture = gl.createTexture();
        gl.bindTexture(gl.TEXTURE_2D, texture);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
        textureRef.current = texture;

        gl.clearColor(0, 0, 0, 0);
        gl.enable(gl.BLEND);
        // Use standard blending for premultiplied alpha result
        gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);

        return () => {
            gl.deleteProgram(program);
            gl.deleteShader(vs);
            gl.deleteShader(fs);
            gl.deleteBuffer(vbo);
            gl.deleteTexture(texture);
        };
    }, []);

    // 2. Poll for spectrum data (Per-Track) - WebGL Rendering Loop
    const spectrumRef = useRef<Float32Array | null>(null);
    useEffect(() => {
        let active = true;
        let animationFrameId: number;

        const loop = async () => {
            if (!active) return;
            try {
                const data = await invoke<number[]>("get_analyzer_data", { trackIdx: trackId });
                if (active && data && data.length > 0) {
                    const byteArray = new Uint8Array(data);
                    const floatArr = new Float32Array(byteArray.buffer);
                    spectrumRef.current = floatArr;
                    
                    // Render to WebGL
                    if (glRef.current && textureRef.current && floatArr.length > 0) {
                        const gl = glRef.current;
                        
                        // Map magnitude directly into alpha channel of unsigned byte texture
                        // We assume Float32 values are dB or amplitudes that we scale
                        // In old CPU: `h = (mag + 80) / 80` -> mag from -80 to 0 mostly.
                        const texData = new Uint8Array(floatArr.length);
                        for (let i = 0; i < floatArr.length; i++) {
                            // Pack magnitude into byte (we decode in shader by not scaling, or just use as is)
                            // We pass mag as is, assuming it will be interpreted as (val/255)
                            // Let's pass magnitude offset so 0 is -80dB, 255 is 0dB
                            const magByte = Math.max(0, Math.min(255, (floatArr[i] + 80) * (255 / 80)));
                            texData[i] = magByte;
                        }

                        gl.bindTexture(gl.TEXTURE_2D, textureRef.current);
                        gl.texImage2D(gl.TEXTURE_2D, 0, gl.ALPHA, floatArr.length, 1, 0, gl.ALPHA, gl.UNSIGNED_BYTE, texData);

                        // Adjust viewport
                        gl.viewport(0, 0, gl.canvas.width, gl.canvas.height);
                        gl.clear(gl.COLOR_BUFFER_BIT);
                        gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
                    }
                }
            } catch (e) {
                // Ignore
            }
            if (active) {
                animationFrameId = requestAnimationFrame(loop);
            }
        };
        
        // Start loop
        loop();
        
        return () => { 
            active = false; 
            if(animationFrameId) cancelAnimationFrame(animationFrameId);
        };
    }, [trackId]);

    // 1. Draw Grid (Static)
    const drawGrid = useCallback(() => {
        const canvas = gridCanvasRef.current;
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        const { width, height } = canvas;
        ctx.clearRect(0, 0, width, height);

        // Horizontal lines (dB)
        ctx.strokeStyle = '#2a2a2e'; // Subtle subtle grid
        ctx.lineWidth = 1;
        ctx.beginPath();
        for (let db = GAIN_MIN; db <= GAIN_MAX; db += 6) {
            const y = gainToY(db, height);
            ctx.moveTo(0, y);
            ctx.lineTo(width, y);
            ctx.fillStyle = '#555';
            ctx.font = '9px Inter';
            if (db !== 0) ctx.fillText(`${db}`, 5, y - 2);
        }
        ctx.stroke();

        // Vertical lines (Freq - Logarithmic)
        const freqs = [20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000];
        ctx.beginPath();
        freqs.forEach(f => {
            const x = freqToX(f, width);
            ctx.moveTo(x, 0);
            ctx.lineTo(x, height);
            const label = f >= 1000 ? `${f / 1000}k` : `${f}`;
            ctx.fillText(label, x + 2, height - 20);
        });
        ctx.stroke();

        // Center Zero line (Brighter)
        ctx.strokeStyle = '#444';
        ctx.lineWidth = 1;
        ctx.beginPath();
        const zeroY = gainToY(0, height);
        ctx.moveTo(0, zeroY);
        ctx.lineTo(width, zeroY);
        ctx.stroke();

        // Piano Roll Helper (Bottom) - "Musical Note Overlay"
        const C_FREQS = [32.7, 65.4, 130.8, 261.6, 523.3, 1046.5, 2093.0, 4186.0, 8372.0];
        ctx.fillStyle = 'rgba(255, 255, 255, 0.03)';
        ctx.fillRect(0, height - 16, width, 16);

        ctx.fillStyle = '#666';
        ctx.textAlign = 'center';
        ctx.font = '10px sans-serif';
        C_FREQS.forEach((f, i) => {
            const x = freqToX(f, width);
            // Key marker
            ctx.fillStyle = 'rgba(255, 255, 255, 0.2)';
            ctx.fillRect(x, height - 16, 1, 16);

            // Label
            if (x < width - 15) {
                ctx.fillStyle = '#888';
                ctx.fillText(`C${i + 1}`, x + 8, height - 4);
            }
        });

    }, []);

    // Helper to calculate response for a frequency
    const getResponseAt = (f: number, bands: EqBand[], modeFilter: 'Stereo' | 'Mid' | 'Side') => {
        let totalGain = 0;
        for (const b of bands) {
            let include = false;
            if (b.mode === 'Stereo') include = true;
            else if (b.mode === 'Mid' && modeFilter === 'Mid') include = true;
            else if (b.mode === 'Side' && modeFilter === 'Side') include = true;

            if (!b.enabled || !include) continue;

            if (b.filter_type === 'Bell') {
                const width = 1 / b.q;
                const octaves = Math.log2(f / b.freq);
                const falloff = Math.exp(-(octaves * octaves) / (width * 0.5));
                totalGain += b.gain_db * falloff;
            } else if (b.filter_type === 'LowShelf') {
                if (f < b.freq) totalGain += b.gain_db;
                else {
                    const octaves = Math.log2(f / b.freq);
                    if (octaves < 1) totalGain += b.gain_db * (1 - octaves);
                }
            } else if (b.filter_type === 'HighShelf') {
                if (f > b.freq) totalGain += b.gain_db;
                else {
                    const octaves = Math.log2(b.freq / f);
                    if (octaves < 1) totalGain += b.gain_db * (1 - octaves);
                }
            } else if (b.filter_type === 'LowPass') {
                const octaves = Math.log2(f / b.freq);
                if (octaves > 0) totalGain -= octaves * 12; // 12dB/oct
            } else if (b.filter_type === 'HighPass') {
                const octaves = Math.log2(b.freq / f);
                if (octaves > 0) totalGain -= octaves * 12; // 12dB/oct
            } else if (b.filter_type === 'Notch') {
                const width = 0.1 / b.q;
                const octaves = Math.abs(Math.log2(f / b.freq));
                if (octaves < width) totalGain -= 40 * (1 - octaves / width);
            }
        }
        return totalGain;
    };

    const renderCurve = (ctx: CanvasRenderingContext2D, width: number, height: number, bands: EqBand[], mode: 'Stereo' | 'Mid' | 'Side') => {
        // "Prisma" Style Colors
        let strokeColor = '#00ffed'; // Default Cyberpunk Cyan
        let glowColor = 'rgba(0, 255, 237, 0.4)';

        if (mode === 'Mid') {
            strokeColor = '#ffaa00'; // Warm Gold
            glowColor = 'rgba(255, 170, 0, 0.4)';
        } else if (mode === 'Side') {
            strokeColor = '#ff00ff'; // Neon Magenta
            glowColor = 'rgba(255, 0, 255, 0.4)';
        }

        ctx.save();
        ctx.beginPath();
        let started = false;

        for (let x = 0; x < width; x += 3) {
            const f = xToFreq(x, width);
            const gain = getResponseAt(f, bands, mode);
            const y = gainToY(gain, height);
            if (!started) {
                ctx.moveTo(x, y);
                started = true;
            } else {
                ctx.lineTo(x, y);
            }
        }

        // "The Expensive Look" - Glow
        ctx.shadowBlur = 15;
        ctx.shadowColor = strokeColor;
        ctx.strokeStyle = strokeColor;
        ctx.lineWidth = 3; // "Energy Beam" thickness
        ctx.lineJoin = 'round';
        ctx.lineCap = 'round';
        ctx.stroke();

        // Optional fill body
        ctx.lineTo(width, height);
        ctx.lineTo(0, height);
        ctx.fillStyle = glowColor.replace('0.4)', '0.05)'); // Very subtle fill
        ctx.fill();

        ctx.restore();
    };

    // Draw Spectrum (Replaced by WebGL above, leaving empty stub for resize)
    const drawSpectrum = useCallback(() => {
        // Handled by WebGL RequestAnimationFrame loop.
        // We trigger resize in the canvas natively.
    }, []);

    // Draw Static Curve (Only on changes)
    const drawCurve = useCallback(() => {
        const canvas = curveCanvasRef.current;
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        const { width, height } = canvas;
        ctx.clearRect(0, 0, width, height);

        // --- Draw EQ Curves (Prisma Mode) ---
        const activeBands = bands.filter(b => b.enabled);
        const hasMid = activeBands.some(b => b.mode === 'Mid');
        const hasSide = activeBands.some(b => b.mode === 'Side');

        if (hasMid || hasSide) {
            renderCurve(ctx, width, height, activeBands, 'Mid');
            renderCurve(ctx, width, height, activeBands, 'Side');
        } else {
            renderCurve(ctx, width, height, activeBands, 'Stereo');
        }

        // Draw Points
        bands.forEach(band => {
            const x = freqToX(band.freq, width);
            const y = gainToY(band.gain_db, height);
            const isSelected = selectedBandId === band.id;
            const isHovered = hoveredBandId === band.id;

            let pointColor = '#00ffed';
            if (band.mode === 'Mid') pointColor = '#ffaa00';
            if (band.mode === 'Side') pointColor = '#ff00ff';

            ctx.fillStyle = isSelected ? '#fff' : (isHovered ? pointColor : pointColor + '88');
            ctx.beginPath();
            ctx.arc(x, y, isSelected ? 6 : 4, 0, Math.PI * 2);
            ctx.fill();

            if (isSelected) {
                ctx.save();
                ctx.shadowBlur = 10;
                ctx.shadowColor = pointColor;
                ctx.strokeStyle = '#fff';
                ctx.lineWidth = 1;
                ctx.beginPath();
                ctx.arc(x, y, 10, 0, Math.PI * 2);
                ctx.stroke();
                ctx.restore();
            }
        });
    }, [bands, selectedBandId, hoveredBandId]);

    useEffect(() => {
        const handleResize = () => {
            if (containerRef.current) {
                const { offsetWidth, offsetHeight } = containerRef.current;
                [gridCanvasRef, spectrumCanvasRef, curveCanvasRef].forEach(ref => {
                    if (ref.current) {
                        ref.current.width = offsetWidth;
                        ref.current.height = offsetHeight;
                    }
                });
                drawGrid();
                drawCurve();
            }
        };

        handleResize();
        window.addEventListener('resize', handleResize);
        return () => window.removeEventListener('resize', handleResize);
    }, [drawGrid, drawCurve]);

    // (The requestAnimationFrame loop is now handled entirely within the WebGL useEffect)

    useEffect(() => {
        drawCurve();
    }, [drawCurve]);

    const handleMouseDown = (e: React.MouseEvent) => {
        const rect = curveCanvasRef.current?.getBoundingClientRect();
        if (!rect) return;
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;

        // Check if clicked on a band
        const width = rect.width;
        const height = rect.height;
        const clickedBand = bands.find(b => {
            const bx = freqToX(b.freq, width);
            const by = gainToY(b.gain_db, height);
            return Math.sqrt((bx - x) ** 2 + (by - y) ** 2) < 15;
        });

        if (clickedBand) {
            setSelectedBandId(clickedBand.id);
            setIsDragging(true);
        } else {
            setSelectedBandId(null);
        }
    };

    const handleMouseMove = (e: React.MouseEvent) => {
        const rect = curveCanvasRef.current?.getBoundingClientRect();
        if (!rect) return;
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;
        setMousePos({ x, y });

        const width = rect.width;
        const height = rect.height;

        if (isDragging && selectedBandId) {
            setBands(prev => prev.map(b => {
                if (b.id === selectedBandId) {
                    const updatedBand = {
                        ...b,
                        freq: xToFreq(x, width),
                        gain_db: yToGain(y, height)
                    };
                    updateBandBackend(updatedBand);
                    return updatedBand;
                }
                return b;
            }));
        } else {
            const hovered = bands.find(b => {
                const bx = freqToX(b.freq, width);
                const by = gainToY(b.gain_db, height);
                return Math.sqrt((bx - x) ** 2 + (by - y) ** 2) < 15;
            });
            setHoveredBandId(hovered?.id || null);
        }
    };

    const handleMouseUp = () => {
        setIsDragging(false);
    };

    const handleDoubleClick = (e: React.MouseEvent) => {
        const rect = curveCanvasRef.current?.getBoundingClientRect();
        if (!rect) return;
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;
        const width = rect.width;
        const height = rect.height;

        const clickedBand = bands.find(b => {
            const bx = freqToX(b.freq, width);
            const by = gainToY(b.gain_db, height);
            return Math.sqrt((bx - x) ** 2 + (by - y) ** 2) < 15;
        });

        if (clickedBand) {
            setBands(prev => prev.map(b => {
                if (b.id === clickedBand.id) {
                    const updated = { ...b, gain_db: 0.0 };
                    updateBandBackend(updated);
                    return updated;
                }
                return b;
            }));
        } else {
            const newBand: EqBand = {
                id: crypto.randomUUID(),
                enabled: true,
                filter_type: 'Bell',
                freq: xToFreq(x, width),
                gain_db: yToGain(y, height),
                q: 1.0,
                mode: 'Stereo',
                solo: false
            };
            setBands(prev => [...prev, newBand]);
            setSelectedBandId(newBand.id);
            updateBandBackend(newBand);
        }
    };

    const handleWheel = (e: React.WheelEvent) => {
        if (selectedBandId) {
            setBands(prev => prev.map(b => {
                if (b.id === selectedBandId) {
                    const delta = e.deltaY > 0 ? 0.9 : 1.1;
                    const updatedBand = { ...b, q: Math.max(0.1, Math.min(10, b.q * delta)) };
                    updateBandBackend(updatedBand);
                    return updatedBand;
                }
                return b;
            }));
        }
    };

    interface EqPreset {
        name: string;
        bands: EqBand[];
    }

    const [presets, setPresets] = useState<EqPreset[]>([]);

    useEffect(() => {
        invoke<EqPreset[]>('get_eq_presets')
            .then(setPresets)
            .catch(err => console.error("Failed to fetch presets:", err));
    }, []);

    const handlePresetChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
        const presetName = e.target.value;
        const preset = presets.find(p => p.name === presetName);
        if (preset) {
            setBands(preset.bands);
            invoke('set_eq_bands', {
                trackIdx: trackId,
                processorId: processorId,
                bands: preset.bands
            }).catch(e => console.error("Failed to set preset:", e));
        }
    };

    return (
        <div className="eq-container" ref={containerRef}>
            <div className="eq-header-controls">
                <select className="eq-preset-select" onChange={handlePresetChange} defaultValue="">
                    <option value="" disabled>Presets</option>
                    {presets.map(p => (
                        <option key={p.name} value={p.name}>{p.name}</option>
                    ))}
                </select>

                {selectedBandId && (
                    <div className="eq-selected-band-panel">
                        <select
                            value={bands.find(b => b.id === selectedBandId)?.filter_type}
                            onChange={(e) => {
                                const newType = e.target.value as EqBand['filter_type'];
                                setBands(prev => prev.map(b => {
                                    if (b.id === selectedBandId) {
                                        const updated = { ...b, filter_type: newType };
                                        updateBandBackend(updated);
                                        return updated;
                                    }
                                    return b;
                                }));
                            }}
                        >
                            <option value="LowPass">Low Pass</option>
                            <option value="HighPass">High Pass</option>
                            <option value="Bell">Bell</option>
                            <option value="LowShelf">Low Shelf</option>
                            <option value="HighShelf">High Shelf</option>
                            <option value="Notch">Notch</option>
                        </select>
                        <select
                            value={bands.find(b => b.id === selectedBandId)?.mode}
                            onChange={(e) => {
                                const newMode = e.target.value as EqBand['mode'];
                                setBands(prev => prev.map(b => {
                                    if (b.id === selectedBandId) {
                                        const updated = { ...b, mode: newMode };
                                        updateBandBackend(updated);
                                        return updated;
                                    }
                                    return b;
                                }));
                            }}
                        >
                            <option value="Stereo">Stereo</option>
                            <option value="Mid">Mid</option>
                            <option value="Side">Side</option>
                            <option value="Left">Left</option>
                            <option value="Right">Right</option>
                        </select>
                        <button onClick={() => {
                            const newBands = bands.filter(b => b.id !== selectedBandId);
                            setBands(newBands);
                            invoke('set_eq_bands', {
                                trackIdx: trackId,
                                processorId: processorId,
                                bands: newBands
                            }).catch(e => console.error("Failed to delete band:", e));
                            setSelectedBandId(null);
                        }}>Delete</button>
                    </div>
                )}
            </div>
            <canvas ref={gridCanvasRef} className="eq-canvas-layer grid" />
            <canvas ref={spectrumCanvasRef} className="eq-canvas-layer spectrum" />
            <canvas ref={curveCanvasRef}
                className="eq-canvas-layer curve"
                onMouseDown={handleMouseDown}
                onMouseMove={handleMouseMove}
                onMouseUp={handleMouseUp}
                onDoubleClick={handleDoubleClick}
                onWheel={handleWheel}
            />
            <div className="eq-tooltip" style={{
                left: mousePos.x + 10,
                top: mousePos.y + 10,
                display: mousePos.x === 0 ? 'none' : 'block'
            }}>
                {Math.round(xToFreq(mousePos.x, curveCanvasRef.current?.width || 1))} Hz ({freqToNote(xToFreq(mousePos.x, curveCanvasRef.current?.width || 1))}) / {yToGain(mousePos.y, curveCanvasRef.current?.height || 1).toFixed(1)} dB
            </div>
        </div>
    );
};
