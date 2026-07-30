import * as React from 'react';
import { invoke } from '@tauri-apps/api/core';

interface TrackLevel {
    id: string;
    peaks: number[];     // [peak_l, peak_r] in linear
    rms: number[];       // [rms_l, rms_r] in linear
    true_peaks: number[];
    lufs_momentary: number;
}

interface TrackVUMeterProps {
    trackId: string;
    color: string;
    height: number;
}

// Compact stereo VU meter that polls get_track_levels.
// Uses a module-level cache so all meters share one poll.
let gLevels: Map<string, TrackLevel> = new Map();
let gListeners: Set<() => void> = new Set();
let gPollTimer: ReturnType<typeof setInterval> | null = null;

function startPolling() {
    if (gPollTimer !== null) return;
    gPollTimer = setInterval(async () => {
        try {
            const raw = await invoke<TrackLevel[]>('get_track_levels');
            const updated = new Map<string, TrackLevel>();
            for (const lvl of raw) updated.set(lvl.id, lvl);
            gLevels = updated;
            for (const fn of gListeners) fn();
        } catch { /* engine not initialised yet — ignore */ }
    }, 50); // 20 Hz refresh
}
function stopPolling() {
    if (gPollTimer !== null) { clearInterval(gPollTimer); gPollTimer = null; }
}

function linToDb(lin: number): number {
    if (lin <= 0) return -144;
    return 20 * Math.log10(lin);
}
function dbToPercent(db: number, min = -60, max = 6): number {
    return Math.max(0, Math.min(100, ((db - min) / (max - min)) * 100));
}
function meterColor(db: number): string {
    if (db > -3) return '#ff2222';
    if (db > -9) return '#ffaa00';
    if (db > -18) return '#aaee44';
    return '#44cc88';
}

export const TrackVUMeter: React.FC<TrackVUMeterProps> = ({ trackId, color: _color, height }) => {
    const [level, setLevel] = React.useState<TrackLevel | null>(null);

    React.useEffect(() => {
        const cb = () => setLevel(gLevels.get(trackId) ?? null);
        gListeners.add(cb);
        startPolling();
        return () => {
            gListeners.delete(cb);
            if (gListeners.size === 0) stopPolling();
        };
    }, [trackId]);

    const peakL = linToDb(level?.peaks[0] ?? 0);
    const peakR = linToDb(level?.peaks[1] ?? 0);
    const rmsL = linToDb(level?.rms[0] ?? 0);
    const rmsR = linToDb(level?.rms[1] ?? 0);

    const barH = Math.max(0, height - 8);

    const Bar = ({ db, rms, side }: { db: number; rms: number; side: 'l' | 'r' }) => {
        const pctPeak = dbToPercent(db);
        const pctRms = dbToPercent(rms);
        const clip = db > -0.1;
        return (
            <div className={`vu-bar-track ${side}`} style={{ height: `${barH}px` }}>
                <div className="vu-bar-bg">
                    {/* RMS fill */}
                    <div className="vu-rms-fill"
                        style={{ height: `${pctRms}%`, background: meterColor(rms) }} />
                    {/* Peak tick */}
                    <div className="vu-peak-tick"
                        style={{ bottom: `${pctPeak}%`, background: clip ? '#ff2222' : 'rgba(255,255,255,0.8)' }} />
                </div>
            </div>
        );
    };

    return (
        <div className="track-vu-meter" title={`L: ${peakL.toFixed(1)} dB  R: ${peakR.toFixed(1)} dB`}>
            <Bar db={peakL} rms={rmsL} side="l" />
            <Bar db={peakR} rms={rmsR} side="r" />
        </div>
    );
};
