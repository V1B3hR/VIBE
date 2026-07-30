import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./AudioSettings.css";

interface AudioDeviceInfo {
    id: string;
    name: string;
    host: string;
    is_default: boolean;
    supported_sample_rates: number[];
    max_input_channels: number;
    max_output_channels: number;
}

interface AudioDeviceConfig {
    host_name: string;
    device_name: string;
    sample_rate: number;
    buffer_size: number;
    input_channels: number;
    output_channels: number;
}

export const AudioSettings = ({ onClose }: { onClose: () => void }) => {
    const [hosts, setHosts] = useState<string[]>([]);
    const [selectedHost, setSelectedHost] = useState<string>("WASAPI");
    const [devices, setDevices] = useState<AudioDeviceInfo[]>([]);
    const [selectedDevice, setSelectedDevice] = useState<string>("");
    const [bufferSizes, setBufferSizes] = useState<number[]>([]);
    const [sampleRates, setSampleRates] = useState<number[]>([]);
    const [config, setConfig] = useState<AudioDeviceConfig>({
        host_name: "WASAPI",
        device_name: "Default",
        sample_rate: 48000,
        buffer_size: 512,
        input_channels: 2,
        output_channels: 2,
    });
    const [uiScale, setUiScale] = useState<number>(1.0);
    const [kropelkaStats, setKropelkaStats] = useState<any>(null);

    useEffect(() => {
        loadAudioSettings();
        // Load UI scale from localStorage
        const savedScale = localStorage.getItem("vibe-ui-scale");
        if (savedScale) {
            const scale = parseFloat(savedScale);
            setUiScale(scale);
            document.documentElement.style.setProperty("--zoom-factor", scale.toString());
        }
    }, []);

    useEffect(() => {
        if (selectedHost) {
            loadDevicesForHost(selectedHost);
        }
    }, [selectedHost]);

    const loadKropelkaStats = async () => {
        try {
            const stats = await invoke<any>("get_kropelka_stats");
            setKropelkaStats(stats);
        } catch (error) {
            console.error("Failed to load Kropelka stats:", error);
        }
    };

    const handleBrainwash = async () => {
        if (window.confirm("Are you sure you want to wipe Kropelka's memory? This cannot be undone.")) {
            try {
                await invoke("reset_kropelka_memory");
                setKropelkaStats(null); // Clear locally to show zero
                loadKropelkaStats(); // Reload to get fresh defaults map
                alert("Kropelka's mind has been wiped clean. 🧼");
            } catch (error) {
                console.error("Failed to brainwash:", error);
                alert("Failed to wipe memory.");
            }
        }
    };

    const loadAudioSettings = async () => {
        try {
            const hostsData = await invoke<string[]>("get_audio_hosts");
            setHosts(hostsData);

            const bufferSizesData = await invoke<number[]>("get_buffer_sizes");
            setBufferSizes(bufferSizesData);

            const sampleRatesData = await invoke<number[]>("get_sample_rates");
            setSampleRates(sampleRatesData);

            const currentConfig = await invoke<AudioDeviceConfig>("get_current_audio_config");
            setConfig(currentConfig);
            setSelectedHost(currentConfig.host_name);

            // Also load stats
            loadKropelkaStats();
        } catch (error) {
            console.error("Failed to load audio settings:", error);
        }
    };

    const loadDevicesForHost = async (host: string) => {
        try {
            const devicesData = await invoke<AudioDeviceInfo[]>("get_audio_devices", {
                hostName: host,
            });
            setDevices(devicesData);
            if (devicesData.length > 0) {
                setSelectedDevice(devicesData[0].id);
            }
        } catch (error) {
            console.error("Failed to load devices:", error);
        }
    };

    const handleApply = async () => {
        try {
            const selectedDeviceInfo = devices.find((d) => d.id === selectedDevice);
            if (!selectedDeviceInfo) return;

            const newConfig: AudioDeviceConfig = {
                ...config,
                host_name: selectedHost,
                device_name: selectedDeviceInfo.name,
            };

            await invoke("set_audio_config", { config: newConfig });
            alert("Audio settings applied! Restart the application for changes to take effect.");
            onClose();
        } catch (error) {
            console.error("Failed to apply audio settings:", error);
            alert("Failed to apply settings: " + error);
        }
    };

    const getLatencyMs = (bufferSize: number, sampleRate: number): number => {
        return (bufferSize / sampleRate) * 1000;
    };

    return (
        <div className="audio-settings-overlay" onClick={onClose}>
            <div className="audio-settings-modal" onClick={(e) => e.stopPropagation()}>
                <div className="audio-settings-header">
                    <h2>⚙️ Audio Settings</h2>
                    <button className="close-btn" onClick={onClose}>
                        ✕
                    </button>
                </div>

                <div className="audio-settings-content">
                    {/* Host Selection */}
                    <div className="setting-group">
                        <label htmlFor="driver-select">Audio Driver</label>
                        <select
                            id="driver-select"
                            value={selectedHost}
                            onChange={(e) => setSelectedHost(e.target.value)}
                            className="setting-select"
                        >
                            {hosts.map((host) => (
                                <option key={host} value={host}>
                                    {host}
                                    {host === "ASIO" && " (Recommended for low latency)"}
                                </option>
                            ))}
                        </select>
                        <span className="setting-hint">
                            ASIO provides the lowest latency on Windows
                        </span>
                    </div>

                    {/* Device Selection */}
                    <div className="setting-group">
                        <label htmlFor="device-select">Audio Device</label>
                        <select
                            id="device-select"
                            value={selectedDevice}
                            onChange={(e) => setSelectedDevice(e.target.value)}
                            className="setting-select"
                        >
                            {devices.map((device) => (
                                <option key={device.id} value={device.id}>
                                    {device.name}
                                </option>
                            ))}
                        </select>
                    </div>

                    {/* Sample Rate */}
                    <div className="setting-group">
                        <label htmlFor="sample-rate-select">Sample Rate</label>
                        <select
                            id="sample-rate-select"
                            value={config.sample_rate}
                            onChange={(e) =>
                                setConfig({ ...config, sample_rate: parseInt(e.target.value) })
                            }
                            className="setting-select"
                        >
                            {sampleRates.map((rate) => (
                                <option key={rate} value={rate}>
                                    {rate} Hz
                                </option>
                            ))}
                        </select>
                    </div>

                    {/* Buffer Size */}
                    <div className="setting-group">
                        <label htmlFor="buffer-size-select">Buffer Size</label>
                        <select
                            id="buffer-size-select"
                            value={config.buffer_size}
                            onChange={(e) =>
                                setConfig({ ...config, buffer_size: parseInt(e.target.value) })
                            }
                            className="setting-select"
                        >
                            {bufferSizes.map((size) => (
                                <option key={size} value={size}>
                                    {size} samples (~
                                    {(getLatencyMs(size, config.sample_rate) ?? 0).toFixed(1)}ms)
                                </option>
                            ))}
                        </select>
                        <span className="setting-hint">
                            Lower = less latency, higher CPU usage
                        </span>
                    </div>

                    {/* UI Scale */}
                    <div className="setting-group">
                        <label>UI Scale: {Math.round(uiScale * 100)}%</label>
                        <input
                            type="range"
                            min="0.5"
                            max="2.0"
                            step="0.05"
                            value={uiScale}
                            onChange={(e) => {
                                const newScale = parseFloat(e.target.value);
                                setUiScale(newScale);
                                document.documentElement.style.setProperty("--zoom-factor", newScale.toString());
                                localStorage.setItem("vibe-ui-scale", newScale.toString());
                            }}
                            className="setting-slider"
                        />
                    </div>

                    {/* Latency Display */}
                    <div className="latency-display">
                        <div className="latency-value">
                            {(getLatencyMs(config.buffer_size, config.sample_rate) ?? 0).toFixed(2)}ms
                        </div>
                        <div className="latency-label">Estimated Latency</div>
                    </div>
                    {/* Kropelka's Mind View */}
                    <div className="kropelka-mind-panel">
                        <h3>🧠 Kropelka's Mind</h3>
                        {kropelkaStats ? (
                            <div className="mind-stats">
                                <div className="mind-row">
                                    <span>Current Mood:</span>
                                    <strong style={{ color: kropelkaStats.frustration > 0.6 ? '#f00' : '#0f0' }}>
                                        {kropelkaStats.persona_tone || "Professional"}
                                    </strong>
                                </div>
                                <div className="mind-row">
                                    <span>Frustration Level:</span>
                                    <div className="progress-bar-bg">
                                        <div className="progress-bar-fill" style={{ width: `${(kropelkaStats.frustration || 0) * 100}%`, background: `linear-gradient(90deg, #0f0, #f00)` }}></div>
                                    </div>
                                </div>
                                <div className="mind-row">
                                    <span>Mixing Affinity:</span>
                                    <div className="progress-bar-bg">
                                        <div className="progress-bar-fill" style={{ width: `${((kropelkaStats.affinities?.Mixing || 0.5) * 100)}%`, background: '#0ff' }}></div>
                                    </div>
                                </div>
                                <div className="mind-row">
                                    <span>Theory Affinity:</span>
                                    <div className="progress-bar-bg">
                                        <div className="progress-bar-fill" style={{ width: `${((kropelkaStats.affinities?.Theory || 0.5) * 100)}%`, background: '#f0f' }}></div>
                                    </div>
                                </div>
                                <div className="mind-actions">
                                    <button className="btn-brainwash" onClick={handleBrainwash}>
                                        🧽 Brainwash (Reset Memory)
                                    </button>
                                </div>
                            </div>
                        ) : (
                            <div className="mind-stats">Loading neural data...</div>
                        )}
                    </div>

                    {/* ─── Keyboard Shortcuts Reference ─────────────────── */}
                    <div className="shortcuts-panel">
                        <h3 className="shortcuts-title">⌨️ Keyboard Shortcuts</h3>
                        <div className="shortcuts-grid">
                            <div className="shortcut-group">
                                <div className="shortcut-group-label">Transport</div>
                                <div className="shortcut-row"><kbd>Space</kbd><span>Play / Pause</span></div>
                                <div className="shortcut-row"><kbd>Shift</kbd>+<kbd>Space</kbd><span>Play from start</span></div>
                                <div className="shortcut-row"><kbd>Home</kbd><span>Jump to start</span></div>
                                <div className="shortcut-row"><kbd>End</kbd><span>Jump to end</span></div>
                                <div className="shortcut-row"><kbd>L</kbd><span>Toggle loop</span></div>
                                <div className="shortcut-row"><kbd>R</kbd><span>Record</span></div>
                            </div>
                            <div className="shortcut-group">
                                <div className="shortcut-group-label">Timeline Editing</div>
                                <div className="shortcut-row"><kbd>S</kbd><span>Split clip at playhead</span></div>
                                <div className="shortcut-row"><kbd>Delete</kbd><span>Delete selected clips</span></div>
                                <div className="shortcut-row"><kbd>Ctrl</kbd>+<kbd>D</kbd><span>Duplicate selected</span></div>
                                <div className="shortcut-row"><kbd>Ctrl</kbd>+<kbd>Z</kbd><span>Undo</span></div>
                                <div className="shortcut-row"><kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd><span>Redo</span></div>
                                <div className="shortcut-row"><kbd>Ctrl</kbd>+<kbd>A</kbd><span>Select all clips</span></div>
                            </div>
                            <div className="shortcut-group">
                                <div className="shortcut-group-label">View &amp; Navigation</div>
                                <div className="shortcut-row"><kbd>+</kbd> / <kbd>-</kbd><span>Zoom in / out</span></div>
                                <div className="shortcut-row"><kbd>Ctrl</kbd>+<kbd>Scroll</kbd><span>Horizontal zoom</span></div>
                                <div className="shortcut-row"><kbd>F</kbd><span>Toggle follow playback</span></div>
                                <div className="shortcut-row"><kbd>M</kbd><span>Mute selected track</span></div>
                                <div className="shortcut-row"><kbd>Escape</kbd><span>Deselect / Close</span></div>
                                <div className="shortcut-row"><kbd>Tab</kbd><span>Next track</span></div>
                            </div>
                            <div className="shortcut-group">
                                <div className="shortcut-group-label">Mixer &amp; Effects</div>
                                <div className="shortcut-row"><kbd>Ctrl</kbd>+<kbd>M</kbd><span>Open Mixer</span></div>
                                <div className="shortcut-row"><kbd>Ctrl</kbd>+<kbd>E</kbd><span>Open Piano Roll</span></div>
                                <div className="shortcut-row"><kbd>Ctrl</kbd>+<kbd>P</kbd><span>Plugin Browser</span></div>
                                <div className="shortcut-row"><kbd>Ctrl</kbd>+<kbd>S</kbd><span>Save project</span></div>
                                <div className="shortcut-row"><kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>E</kbd><span>Export</span></div>
                                <div className="shortcut-row"><kbd>Ctrl</kbd>+<kbd>,</kbd><span>Audio Settings</span></div>
                            </div>
                        </div>
                    </div>
                </div>

                <div className="audio-settings-footer">
                    <button className="btn-cancel" onClick={onClose}>
                        Cancel
                    </button>
                    <button className="btn-apply" onClick={handleApply}>
                        Apply Settings
                    </button>
                </div>
            </div>
        </div>
    );
};
