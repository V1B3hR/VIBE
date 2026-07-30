import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Play, Square, Circle, RotateCcw, RotateCw, Activity, Repeat } from "lucide-react";
import "./Transport.css";

export const Transport = () => {
    const [isPlaying, setIsPlaying] = useState(false);
    const [isRecording, setIsRecording] = useState(false);
    const [playhead, setPlayhead] = useState(0);
    const [bpm, setBpm] = useState(120);
    const [isEditingBpm, setIsEditingBpm] = useState(false);
    const [cpuLoad, setCpuLoad] = useState(0);
    const [memoryUsage, setMemoryUsage] = useState(0);
    const [lastStopTime, setLastStopTime] = useState(0); // For pause/reset logic
    const [metronomeEnabled, setMetronomeEnabled] = useState(false);
    const [isLoopEnabled, setIsLoopEnabled] = useState(false);

    const fetchState = async () => {
        try {
            const playing = await invoke<boolean>("is_playing");
            const recording = await invoke<boolean>("is_recording");
            const ph = await invoke<number>("get_playhead");
            const currentBpm = await invoke<number>("get_bpm");
            const cpu = await invoke<number>("get_cpu_load");
            const mem = await invoke<number>("get_memory_usage");

            const looping = await invoke<boolean>("is_loop_enabled");

            setIsPlaying(playing);
            setIsRecording(recording);
            setPlayhead(ph);
            setBpm(currentBpm);
            setCpuLoad(cpu);
            setMemoryUsage(mem);
            setIsLoopEnabled(looping);
        } catch (e) {
            console.error(e);
        }
    };

    useEffect(() => {
        const interval = setInterval(fetchState, 50); // Faster polling for smooth meters
        return () => clearInterval(interval);
    }, []);

    const togglePlay = async () => {
        if (isPlaying) {
            await invoke("pause_audio");
        } else {
            await invoke("play_audio");
        }
    };

    const toggleRecord = async () => {
        await invoke("toggle_record");
    };

    const toggleMetronome = async () => {
        const newState = !metronomeEnabled;
        setMetronomeEnabled(newState);
        await invoke("set_metronome", { enabled: newState });
    };

    const toggleLoop = async () => {
        const newState = !isLoopEnabled;
        setIsLoopEnabled(newState);
        await invoke("set_loop_enabled", { enabled: newState });
    };

    const stop = async () => {
        const now = Date.now();
        if (now - lastStopTime < 300) {
            // Double click - reset to start
            await invoke("stop_transport");
            await invoke("set_playhead", { sample: 0 });
            console.log("⏮️ Reset to start");
        } else {
            // Single click - pause
            await invoke("pause_audio");
            console.log("⏸️ Paused");
        }
        setLastStopTime(now);
    };

    const undo = async () => {
        await invoke("undo");
    };

    const redo = async () => {
        await invoke("redo");
    };

    const formatTime = (samples: number) => {
        const sampleRate = 44100;
        const totalSeconds = samples / sampleRate;
        const mins = Math.floor(totalSeconds / 60);
        const secs = Math.floor(totalSeconds % 60);
        const ms = Math.floor((totalSeconds % 1) * 100);
        return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}.${ms.toString().padStart(2, '0')}`;
    };

    const formatBars = (samples: number, currentBpm: number) => {
        const samplesPerBeat = (44100 * 60) / currentBpm;
        const totalBeats = samples / samplesPerBeat;
        const bar = Math.floor(totalBeats / 4) + 1;
        const beat = Math.floor(totalBeats % 4) + 1;
        return `${bar}.${beat}`;
    };

    const handleBpmMouseDown = (e: React.MouseEvent) => {
        if (isEditingBpm) return;
        const startY = e.clientY;
        const startBpm = bpm;

        const onMouseMove = (moveEvent: MouseEvent) => {
            const deltaY = startY - moveEvent.clientY;
            const newBpm = Math.min(Math.max(startBpm + Math.floor(deltaY / 5), 20), 400);
            setBpm(newBpm);
            invoke("set_bpm", { bpm: parseFloat((newBpm ?? 120).toFixed(1)) });
        };

        const onMouseUp = () => {
            window.removeEventListener("mousemove", onMouseMove);
            window.removeEventListener("mouseup", onMouseUp);
        };

        window.addEventListener("mousemove", onMouseMove);
        window.addEventListener("mouseup", onMouseUp);
    };

    return (
        <div className="transport-bar">
            {/* LCD Center Panel */}
            <div className="transport-lcd-panel" onDoubleClick={() => setIsEditingBpm(true)}>
                <div className="lcd-time">{formatTime(playhead)}</div>
                <div className="lcd-bottom">
                    {isEditingBpm ? (
                        <input
                            autoFocus
                            data-testid="bpm-input"
                            className="lcd-bpm-input"
                            value={bpm}
                            onBlur={() => setIsEditingBpm(false)}
                            onChange={(e) => {
                                const val = parseFloat(e.target.value);
                                if (!isNaN(val)) {
                                    setBpm(val);
                                    invoke("set_bpm", { bpm: val });
                                }
                            }}
                        />
                    ) : (
                        <span className="lcd-bpm" data-testid="bpm-value">{(bpm ?? 120).toFixed(1)} BPM</span>
                    )}
                </div>
                <div className="lcd-stats">
                    <div className="lcd-meter-mini">
                        <span className="mini-label">CPU</span>
                        <div className="mini-bar-bg" data-testid="cpu-fill-container">
                             <div className={`mini-bar-fill ${(cpuLoad ?? 0) > 80 ? 'danger' : ''}`} style={{ width: `${cpuLoad ?? 0}%` }} data-testid="cpu-fill"></div>
                        </div>
                        <span className="mini-value" data-testid="cpu-value">{(cpuLoad ?? 0).toFixed(1)}%</span>
                    </div>
                    <div className="lcd-meter-mini">
                        <span className="mini-label">MEM</span>
                        <div className="mini-bar-bg">
                             <div className="mini-bar-fill" style={{ width: `${memoryUsage ?? 0}%` }}></div>
                        </div>
                        <span className="mini-value" data-testid="mem-value">{(memoryUsage ?? 0).toFixed(1)}%</span>
                    </div>
                </div>
            </div>

            {/* Transport Action Controls */}
            <div className="transport-button-group">
                <button className={`btn-t-action ${isPlaying ? 'active-play' : ''}`} onClick={togglePlay} data-testid="transport-play">
                    {isPlaying ? <div className="pause-icon" /> : <Play size={20} fill="currentColor" stroke="none" />}
                </button>
                <button className="btn-t-action" onClick={stop} data-testid="transport-stop">
                    <Square size={16} fill="currentColor" stroke="none" />
                </button>
                <button className={`btn-t-action ${isRecording ? 'active-rec' : ''}`} onClick={toggleRecord} data-testid="transport-record">
                    <Circle size={16} fill={isRecording ? "#ef4444" : "currentColor"} stroke="none" />
                </button>
                
                <div className="divider-v"></div>
                
                <button className={`btn-t-action secondary ${isLoopEnabled ? 'active' : ''}`} onClick={toggleLoop} data-testid="transport-loop" title="Loop">
                    <Repeat size={16} color={isLoopEnabled ? "#00f0ff" : "currentColor"} />
                </button>
                <button className={`btn-t-action secondary ${metronomeEnabled ? 'active' : ''}`} onClick={toggleMetronome} data-testid="transport-metronome" title="Metronome">
                    <Activity size={16} color={metronomeEnabled ? "#00f0ff" : "currentColor"} />
                </button>

                <div className="divider-v"></div>

                <button className="btn-t-action secondary" onClick={undo} title="Undo" data-testid="transport-undo">
                    <RotateCcw size={16} />
                </button>
                <button className="btn-t-action secondary" onClick={redo} title="Redo" data-testid="transport-redo">
                    <RotateCw size={16} />
                </button>
            </div>

            {/* GPU / CPU Meter Panel */}
            <div className="transport-meter-panel">
                <div className="meter-header">
                    <span className="meter-label">GPU</span>
                    <div className="meter-display">
                        <div className="meter-leds">
                            {[...Array(10)].map((_, i) => (
                                <div key={i} className={`led ${cpuLoad > i * 10 ? (i > 7 ? 'danger' : i > 5 ? 'warning' : 'active') : ''}`} />
                            ))}
                        </div>
                    </div>
                </div>
                <div className="meter-header">
                    <span className="meter-label">L</span>
                    <span className="meter-label">R</span>
                </div>
            </div>
        </div>
    );
};
