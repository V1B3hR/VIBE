import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./SpectrumAnalyzer.css";

interface SpectrumAnalyzerProps {
    trackId?: number;
}

export const SpectrumAnalyzer: React.FC<SpectrumAnalyzerProps> = ({ trackId = 0 }) => {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const spectrumDataRef = useRef<Float32Array>(new Float32Array(2048).fill(0));
    const animationFrameId = useRef<number | undefined>(undefined);
    
    // WebGL refs
    const glRef = useRef<WebGLRenderingContext | null>(null);
    const programRef = useRef<WebGLProgram | null>(null);
    const textureRef = useRef<WebGLTexture | null>(null);

    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;

        const gl = canvas.getContext("webgl", { alpha: true });
        if (!gl) return;
        glRef.current = gl;

        // Vertex Shader
        const vsSource = `
            attribute vec2 a_position;
            varying vec2 v_uv;
            void main() {
                v_uv = a_position * 0.5 + 0.5;
                gl_Position = vec4(a_position, 0.0, 1.0);
            }
        `;

        // Fragment Shader: GPU Spectral Engine! Evaluates DFT per-pixel
        const fsSource = `
            precision highp float;
            varying vec2 v_uv;
            uniform sampler2D u_tex;
            #define FFT_SIZE 2048.0
            #define PI 3.14159265359

            void main() {
                // Map v_uv.x (0.0 -> 1.0) to a frequency in Hz (20Hz to 20kHz log scale)
                float minFreq = 20.0;
                float maxFreq = 20000.0;
                float sampleRate = 48000.0;
                
                float f = minFreq * pow(maxFreq / minFreq, v_uv.x);
                float k = f * FFT_SIZE / sampleRate;

                float real = 0.0;
                float imag = 0.0;
                
                // Unroll loop for WebGL 1.0 safely
                for(int i = 0; i < 2048; i++) {
                    float n = float(i);
                    float sampleData = texture2D(u_tex, vec2(n / FFT_SIZE, 0.5)).a;
                    // Decode from 0-1 to -1 to +1
                    float x_n = sampleData * 2.0 - 1.0;
                    
                    // Hann Window
                    float window = 0.5 * (1.0 - cos(2.0 * PI * n / (FFT_SIZE - 1.0)));
                    x_n *= window;

                    float angle = -2.0 * PI * k * n / FFT_SIZE;
                    real += x_n * cos(angle);
                    imag += x_n * sin(angle);
                }

                float mag = sqrt(real * real + imag * imag);
                float db = 20.0 * log(max(mag, 1e-6)) / log(10.0);
                
                // Map dB to 0.0 -> 1.0 visualization height (e.g., -80dB to +60dB span)
                float val = (db + 80.0) / 90.0;
                val = clamp(val, 0.0, 1.0);

                // Draw beautiful smooth continuous spectrum
                if (v_uv.y > val) { 
                    float dist = v_uv.y - val;
                    float glow = 0.005 / (dist + 0.01);
                    gl_FragColor = vec4(0.0, 0.94, 1.0, glow * 0.8 * val);
                } else {
                    vec3 colorBottom = vec3(0.0, 0.26, 0.66);
                    vec3 colorMid = vec3(0.0, 1.0, 1.0);
                    vec3 colorTop = vec3(1.0, 0.8, 0.0);
                    
                    vec3 c = mix(colorBottom, colorMid, smoothstep(0.0, 0.4, v_uv.y));
                    c = mix(c, colorTop, smoothstep(0.4, 0.8, v_uv.y));
                    
                    gl_FragColor = vec4(c, 0.95);
                }
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

        // Screen Quad
        const vertices = new Float32Array([
            -1, -1,  1, -1,  -1, 1,  1, 1, // Triangle strip
        ]);
        const vbo = gl.createBuffer();
        gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
        gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.STATIC_DRAW);

        const posAttr = gl.getAttribLocation(program, "a_position");
        gl.enableVertexAttribArray(posAttr);
        gl.vertexAttribPointer(posAttr, 2, gl.FLOAT, false, 0, 0);

        // Create 1D Texture for spectrum data
        const texture = gl.createTexture();
        gl.bindTexture(gl.TEXTURE_2D, texture);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
        textureRef.current = texture;

        gl.clearColor(0.039, 0.039, 0.058, 0.3); // rgba(10, 10, 15, 0.3)
        gl.enable(gl.BLEND);
        gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

        // cleanup function for old WebGL context
        return () => {
            gl.deleteProgram(program);
            gl.deleteShader(vs);
            gl.deleteShader(fs);
            gl.deleteBuffer(vbo);
            gl.deleteTexture(texture);
        };
    }, []);

    useEffect(() => {
        const updateSpectrumData = async () => {
            try {
                const data = await invoke<number[]>("get_analyzer_data", { trackIdx: trackId });
                if (data && data.length > 0) {
                    const byteArray = new Uint8Array(data);
                    const floatArr = new Float32Array(byteArray.buffer);
                    spectrumDataRef.current = floatArr;
                }
            } catch (e) {
                // Ignore
            }
        };

        const renderWebGL = () => {
            if (!glRef.current || !programRef.current || !textureRef.current) return;
            const gl = glRef.current;
            const floatArr = spectrumDataRef.current;

            // Map float audio (-1.0 to 1.0) to Uint8 (0 to 255) for texture upload
            const texData = new Uint8Array(2048);
            for (let i = 0; i < 2048; i++) {
                // Since data might be short if partial buffer, default to 0 if out of bounds
                const sample = floatArr[i] || 0.0;
                texData[i] = Math.min(255, Math.max(0, (sample + 1.0) * 127.5));
            }

            gl.bindTexture(gl.TEXTURE_2D, textureRef.current);
            gl.texImage2D(gl.TEXTURE_2D, 0, gl.ALPHA, 2048, 1, 0, gl.ALPHA, gl.UNSIGNED_BYTE, texData);

            gl.clear(gl.COLOR_BUFFER_BIT);
            gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
        };

        const animate = () => {
            updateSpectrumData();
            renderWebGL();
            animationFrameId.current = requestAnimationFrame(animate);
        };

        animate();

        return () => {
            if (animationFrameId.current) {
                cancelAnimationFrame(animationFrameId.current);
            }
        };
    }, [trackId]);

    return (
        <div className="spectrum-analyzer">
            <div className="spectrum-header" style={{ position: 'absolute', top: '10px', left: '10px', zIndex: 10 }}>
                <span className="spectrum-title">SPECTRUM ANALYZER (GPU)</span>
                <span className="spectrum-badge">WEBGL FFT-2048</span>
            </div>
            <div style={{ position: 'relative', width: '100%', height: '200px' }}>
                <canvas
                    ref={canvasRef}
                    width={800}
                    height={200}
                    className="spectrum-canvas"
                    style={{ position: 'absolute', top: 0, left: 0, width: '100%', height: '100%' }}
                />
                
                {/* HTML Labels overlay for performance (GPU handles the bars) */}
                <div style={{ position: 'absolute', bottom: '0px', left: '0', width: '100%', display: 'flex', justifyContent: 'space-between', padding: '0 2%', boxSizing: 'border-box', pointerEvents: 'none' }}>
                    {["20Hz", "100Hz", "500Hz", "1kHz", "5kHz", "10kHz", "20kHz"].map((label, i) => (
                        <span key={i} style={{ color: '#888', font: "10px 'Inter', sans-serif" }}>{label}</span>
                    ))}
                </div>
            </div>
        </div>
    );
};
