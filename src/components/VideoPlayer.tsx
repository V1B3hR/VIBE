import React, { useRef, useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { Film, X, Maximize2, Settings, Clock, Play, Pause, RotateCcw } from 'lucide-react';
import { convertFileSrc } from '@tauri-apps/api/core';
import './VideoPlayer.css';

interface VideoState {
    path: string | null;
    filename: string | null;
    framerate: number;
    offset_samples: number;
    is_active: boolean;
}

export const VideoPlayer: React.FC<{ onClose: () => void }> = ({ onClose }) => {
    const videoRef = useRef<HTMLVideoElement>(null);
    const [state, setState] = useState<VideoState | null>(null);
    const [showSettings, setShowSettings] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [isHovered, setIsHovered] = useState(false);
    const [currentTime, setCurrentTime] = useState(0);

    const fetchState = useCallback(async () => {
        try {
            const s = await invoke<VideoState>('get_video_state');
            setState(s);
        } catch (e) {
            console.error("Failed to fetch video state", e);
        }
    }, []);

    const handleLoadVideo = async () => {
        try {
            const selected = await open({
                multiple: false,
                filters: [{
                    name: 'Video',
                    extensions: ['mp4', 'm4v', 'webm', 'mov']
                }]
            });

            if (selected) {
                // In Tauri v2, selected path is a string
                const path = Array.isArray(selected) ? selected[0] : selected;
                const newState = await invoke<VideoState>('load_video', { path });
                setState(newState);
                setError(null);
            }
        } catch (e) {
            setError("Failed to load video: " + e);
        }
    };

    const handleSetOffset = async (samples: number) => {
        await invoke('set_video_offset', { offsetSamples: samples });
        fetchState();
    };

    const handleSetFramerate = async (fps: number) => {
        await invoke('set_video_framerate', { fps });
        fetchState();
    };

    // Sync Logic
    useEffect(() => {
        let rafId: number;

        const syncLoop = async () => {
            if (videoRef.current && state?.is_active) {
                try {
                    const playhead = await invoke<number>('get_playhead');
                    const isPlaying = await invoke<boolean>('is_playing');

                    const timeSec = (playhead - state.offset_samples) / 48000.0;
                    setCurrentTime(timeSec);

                    // Scrubbing / Jump detection (0.05s threshold for 20fps-ish sync)
                    const diff = Math.abs(videoRef.current.currentTime - timeSec);
                    if (diff > 0.05) {
                        videoRef.current.currentTime = timeSec;
                    }

                    // Playback sync
                    if (isPlaying && videoRef.current.paused) {
                        videoRef.current.play().catch(() => { });
                    } else if (!isPlaying && !videoRef.current.paused) {
                        videoRef.current.pause();
                    }
                } catch (e) {
                    // Ignore RPC errors during teardown
                }
            }
            rafId = requestAnimationFrame(syncLoop);
        };

        rafId = requestAnimationFrame(syncLoop);
        return () => cancelAnimationFrame(rafId);
    }, [state]);

    useEffect(() => {
        fetchState();
    }, [fetchState]);

    return (
        <div
            className={`video-player-modal ${isHovered ? 'controls-visible' : ''}`}
            onMouseEnter={() => setIsHovered(true)}
            onMouseLeave={() => setIsHovered(false)}
        >
            <div className="video-header">
                <div className="video-title">
                    <Film size={14} className="icon-neon" />
                    <span>{state?.filename || "Video Synchronizer"}</span>
                </div>
                <div className="video-info">
                    <Clock size={12} />
                    <span>{currentTime.toFixed(3)}s</span>
                </div>
                <div className="video-actions">
                    <Settings size={14} onClick={() => setShowSettings(!showSettings)} className="action-icon" />
                    <Maximize2 size={14} className="action-icon" />
                    <X size={14} onClick={onClose} className="action-icon close-icon" />
                </div>
            </div>

            <div className="video-content">
                {state?.path ? (
                    <video
                        ref={videoRef}
                        src={convertFileSrc(state.path)}
                        className="video-element"
                        crossOrigin="anonymous"
                    />
                ) : (
                    <div className="video-placeholder" onClick={handleLoadVideo}>
                        <div className="placeholder-content">
                            <Film size={64} className="placeholder-icon" />
                            <h2>Film Scoring Enabled</h2>
                            <p>Import a video file to synchronize with VIBE transport.</p>
                            <button className="vibe-btn-neon">Load Video File</button>
                        </div>
                    </div>
                )}

                {showSettings && (
                    <div className="video-settings-overlay">
                        <div className="settings-panel">
                            <div className="settings-header">
                                <h3>Synchronization Settings</h3>
                                <X size={16} onClick={() => setShowSettings(false)} className="close-icon" />
                            </div>

                            <div className="settings-body">
                                <div className="setting-row">
                                    <label>Frame Rate</label>
                                    <div className="setting-input-group">
                                        <select
                                            value={state?.framerate}
                                            onChange={(e) => handleSetFramerate(parseFloat(e.target.value))}
                                        >
                                            <option value={23.976}>23.976 fps</option>
                                            <option value={24}>24.000 fps</option>
                                            <option value={25}>25.000 fps</option>
                                            <option value={29.97}>29.970 fps</option>
                                            <option value={30}>30.000 fps</option>
                                            <option value={48}>48.000 fps</option>
                                            <option value={60}>60.000 fps</option>
                                        </select>
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <label>Start Offset (Samples)</label>
                                    <div className="setting-input-group">
                                        <input
                                            type="number"
                                            value={state?.offset_samples}
                                            onChange={(e) => handleSetOffset(parseInt(e.target.value))}
                                        />
                                        <button className="icon-btn" onClick={() => handleSetOffset(0)}>
                                            <RotateCcw size={14} />
                                        </button>
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <label>Media</label>
                                    <button className="vibe-btn-outline full-width" onClick={handleLoadVideo}>
                                        Replace Video Source
                                    </button>
                                </div>
                            </div>
                        </div>
                    </div>
                )}
            </div>

            {error && <div className="video-error">{error}</div>}
        </div>
    );
};
