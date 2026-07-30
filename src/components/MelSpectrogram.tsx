import { useEffect, useRef, useState, useMemo } from 'react';
import './MelSpectrogram.css';

interface MelFrame {
    data: number[];
    timestamp_samples: number;
}

interface MelSpectrogramProps {
    frames: MelFrame[];
    width: number;
    height: number;
    loading?: boolean;
    playhead?: number; // Global playhead in samples
    clipStart?: number; // Clip start in samples
    clipDuration?: number; // Clip duration in samples
}

// WebGPU Constants Shim
const GPUBufferUsage = {
    STORAGE: 0x0080,
    UNIFORM: 0x0040,
    COPY_DST: 0x0008,
};

export const MelSpectrogram = ({ frames, width, height, loading, playhead, clipStart, clipDuration }: MelSpectrogramProps) => {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const containerRef = useRef<HTMLDivElement>(null);
    const [useWebGPU, setUseWebGPU] = useState(true);

    // Zoom & Navigation State
    const [zoom, setZoom] = useState(1.0);
    const [offset, setOffset] = useState(0.0); // 0.0 to 1.0 (start of visible range)

    useEffect(() => {
        const initRender = async () => {
            if (!canvasRef.current || (frames.length === 0 && !loading)) return;

            if ((navigator as any).gpu && useWebGPU) {
                try {
                    await renderWebGPU();
                } catch (e) {
                    console.warn("WebGPU failed, falling back to WebGL2:", e);
                    setUseWebGPU(false);
                    renderWebGL2();
                }
            } else {
                renderWebGL2();
            }
        };

        initRender();
    }, [frames, width, height, useWebGPU, loading, zoom, offset]);

    const renderWebGPU = async () => {
        const canvas = canvasRef.current!;
        const gpu = (navigator as any).gpu;
        const adapter = await gpu.requestAdapter();
        if (!adapter) throw new Error("No GPU adapter found");
        const device = await adapter.requestDevice();

        const context = canvas.getContext('webgpu') as any;
        const format = gpu.getPreferredCanvasFormat();

        context.configure({
            device,
            format,
            alphaMode: 'premultiplied',
        });

        // Prepare Data
        const n_mels = frames[0].data.length;
        const n_frames = frames.length;
        const data = new Float32Array(n_frames * n_mels);

        for (let f = 0; f < n_frames; f++) {
            for (let m = 0; m < n_mels; m++) {
                // Normalize log-magnitude: assume range [-80, 20] -> [0, 1]
                const val = frames[f].data[m];
                data[f * n_mels + m] = (val + 80) / 100;
            }
        }

        const dataBuffer = device.createBuffer({
            size: data.byteLength,
            usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
        });
        device.queue.writeBuffer(dataBuffer, 0, data);

        const shaderModule = device.createShaderModule({
            code: `
                struct VertexOutput {
                    @builtin(position) position: vec4f,
                    @location(0) uv: vec2f,
                }

                @vertex
                fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
                    var positions = array<vec2f, 4>(
                        vec2f(-1., -1.), vec2f(1., -1.), vec2f(-1., 1.), vec2f(1., 1.)
                    );
                    var uvs = array<vec2f, 4>(
                        vec2f(0., 1.), vec2f(1., 1.), vec2f(0., 0.), vec2f(1., 0.)
                    );
                    var out: VertexOutput;
                    out.position = vec4f(positions[idx], 0.0, 1.0);
                    out.uv = uvs[idx];
                    return out;
                }

                @group(0) @binding(0) var<storage, read> spec_data: array<f32>;
                @group(0) @binding(1) var<uniform> config: vec4f; // (n_frames, n_mels, zoom, offset)

                @fragment
                fn fs_main(@location(0) uv: vec2f) -> @location(0) vec4f {
                    let frame_idx = u32(uv.x * config.x);
                    
                    // Apply Zoom & Offset to Y (Frequency)
                    let zoom = config.z;
                    let offset = config.w;
                    let target_uv_y = uv.y / zoom + offset;
                    
                    let mel_idx = u32((1.0 - target_uv_y) * config.y); 
                    let idx = frame_idx * u32(config.y) + mel_idx;
                    
                    if (idx >= arrayLength(&spec_data) || target_uv_y < 0.0 || target_uv_y > 1.0) {
                        return vec4f(0.01, 0.0, 0.02, 1.0); // Deep background
                    }

                    let val = clamp(spec_data[idx], 0.0, 1.0);
                    
                    // Prisma Palette: Deep Purple -> Blue -> Cyan -> Green -> Yellow -> White
                    let color = mix(
                        mix(vec3f(0.05, 0.0, 0.1), vec3f(0.0, 0.5, 1.0), smoothstep(0.0, 0.4, val)),
                        mix(vec3f(0.0, 1.0, 0.8), vec3f(1.0, 1.0, 1.0), smoothstep(0.4, 1.0, val)),
                        smoothstep(0.4, 0.6, val)
                    );
                    
                    return vec4f(color, 1.0);
                }
            `
        });

        const configBuffer = device.createBuffer({
            size: 16,
            usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
        });
        device.queue.writeBuffer(configBuffer, 0, new Float32Array([n_frames, n_mels, zoom, offset]));

        const pipeline = device.createRenderPipeline({
            layout: 'auto',
            vertex: {
                module: shaderModule,
                entryPoint: 'vs_main',
            },
            fragment: {
                module: shaderModule,
                entryPoint: 'fs_main',
                targets: [{ format }],
            },
            primitive: {
                topology: 'triangle-strip',
            },
        });

        const bindGroup = device.createBindGroup({
            layout: pipeline.getBindGroupLayout(0),
            entries: [
                { binding: 0, resource: { buffer: dataBuffer } },
                { binding: 1, resource: { buffer: configBuffer } },
            ],
        });

        const commandEncoder = device.createCommandEncoder();
        const passEncoder = commandEncoder.beginRenderPass({
            colorAttachments: [{
                view: context.getCurrentTexture().createView(),
                clearValue: { r: 0, g: 0, b: 0, a: 1 },
                loadOp: 'clear',
                storeOp: 'store',
            }],
        });

        passEncoder.setPipeline(pipeline);
        passEncoder.setBindGroup(0, bindGroup);
        passEncoder.draw(4);
        passEncoder.end();

        device.queue.submit([commandEncoder.finish()]);
    };

    const renderWebGL2 = () => {
        const canvas = canvasRef.current;
        if (!canvas) return;

        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        if (frames.length === 0) return;

        const n_mels = frames[0].data.length;
        const n_frames = frames.length;

        // Manual zoom implementation for 2D fallback
        const displayedMels = Math.floor(n_mels / zoom);
        const melOffset = Math.floor(offset * n_mels);

        const imgData = ctx.createImageData(n_frames, displayedMels);

        for (let f = 0; f < n_frames; f++) {
            for (let m = 0; m < displayedMels; m++) {
                const sourceMel = m + melOffset;
                if (sourceMel >= n_mels) break;

                const val = (frames[f].data[sourceMel] + 80) / 100;
                const idx = ((displayedMels - 1 - m) * n_frames + f) * 4;

                imgData.data[idx] = Math.max(0, Math.min(255, val * 255));
                imgData.data[idx + 1] = Math.max(0, Math.min(255, val * 200));
                imgData.data[idx + 2] = Math.max(0, Math.min(255, val * 255));
                imgData.data[idx + 3] = 255;
            }
        }

        const tempCanvas = document.createElement('canvas');
        tempCanvas.width = n_frames;
        tempCanvas.height = displayedMels;
        tempCanvas.getContext('2d')?.putImageData(imgData, 0, 0);

        ctx.clearRect(0, 0, width, height);
        ctx.imageSmoothingEnabled = true;
        ctx.drawImage(tempCanvas, 0, 0, width, height);
    };

    const handleWheel = (e: React.WheelEvent) => {
        if (e.ctrlKey) {
            e.preventDefault();
            const delta = e.deltaY > 0 ? 0.9 : 1.1;
            const newZoom = Math.min(Math.max(zoom * delta, 1.0), 20.0);

            // Zoom towards mouse
            const rect = canvasRef.current!.getBoundingClientRect();
            const mouseRelativeY = 1.0 - (e.clientY - rect.top) / rect.height;

            const oldVisibleRange = 1.0 / zoom;
            const newVisibleRange = 1.0 / newZoom;

            const newOffset = Math.min(Math.max(offset + (oldVisibleRange - newVisibleRange) * mouseRelativeY, 0.0), 1.0 - newVisibleRange);

            setZoom(newZoom);
            setOffset(newOffset);
        } else if (e.shiftKey) {
            // Scroll offset
            e.preventDefault();
            const delta = e.deltaY * 0.001;
            const visibleRange = 1.0 / zoom;
            setOffset(Math.min(Math.max(offset + delta, 0.0), 1.0 - visibleRange));
        }
    };

    const freqLabels = useMemo(() => {
        const labels = [
            { f: 60, label: "60 Hz" },
            { f: 100, label: "100 Hz" },
            { f: 250, label: "250 Hz" },
            { f: 500, label: "500 Hz" },
            { f: 1000, label: "1 kHz" },
            { f: 2000, label: "2 kHz" },
            { f: 4000, label: "4 kHz" },
            { f: 8000, label: "8 kHz" },
            { f: 16000, label: "16 kHz" },
        ];

        // Mel scale constants (matching backend)
        const hzToMel = (hz: number) => 2595 * Math.log10(1 + hz / 700);
        const melMin = hzToMel(20);
        const melMax = hzToMel(20000);

        return labels.map(l => {
            const mel = hzToMel(l.f);
            const normalized = (mel - melMin) / (melMax - melMin);
            // Invert because Y typically goes down in canvas but up in UI
            const visibleY = (normalized - offset) * zoom;
            return { ...l, visibleY };
        }).filter(l => l.visibleY >= -0.05 && l.visibleY <= 1.05);
    }, [zoom, offset]);

    // Cursor calculation
    const cursorX = useMemo(() => {
        if (playhead === undefined || clipStart === undefined || clipDuration === undefined) return -1;
        const relPos = (playhead - clipStart) / clipDuration;
        if (relPos < 0 || relPos > 1) return -1;
        return relPos * 100;
    }, [playhead, clipStart, clipDuration]);

    return (
        <div className="mel-spectrogram-container" ref={containerRef} onWheel={handleWheel}>
            <div className="spectrogram-y-axis">
                {freqLabels.map(l => (
                    <div
                        key={l.f}
                        className="axis-label"
                        style={{ bottom: `${l.visibleY * 100}%` }}
                    >
                        {l.label}
                    </div>
                ))}
            </div>
            <canvas
                ref={canvasRef}
                width={width}
                height={height}
                className="mel-spectrogram-canvas"
            />
            {cursorX >= 0 && (
                <div
                    className="spectrogram-cursor"
                    style={{ left: `calc(50px + ${cursorX}% * (1 - 50px / 100%))` }}
                />
            )}
            <div className="spectrogram-overlay">
                SPECTRAL ENGINE v5.0 | {useWebGPU ? 'WebGPU' : 'WebGL2'} | Zoom: {zoom.toFixed(1)}x
            </div>
            {loading && (
                <div className="spectrogram-loading">
                    <div className="spinner"></div>
                    <span>ANALYZING SONIC ARCHITECTURE...</span>
                </div>
            )}
        </div>
    );
};
